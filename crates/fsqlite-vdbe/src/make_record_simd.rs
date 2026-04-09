use fsqlite_types::SqliteValue;
use fsqlite_types::serial_type::{serial_type_for_integer, write_varint};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IntegerEncoding {
    serial_type: u8,
    payload_len: u8,
}

impl IntegerEncoding {
    #[inline]
    fn from_serial_type(serial_type: u8) -> Self {
        let payload_len = match serial_type {
            8 | 9 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            5 => 6,
            6 => 8,
            _ => unreachable!("integer serial type must be in 1..=9"),
        };
        Self {
            serial_type,
            payload_len,
        }
    }
}

#[inline]
fn scalar_integer_encoding(value: i64) -> IntegerEncoding {
    let serial_type = u8::try_from(serial_type_for_integer(value)).unwrap_or(0);
    IntegerEncoding::from_serial_type(serial_type)
}

#[allow(clippy::cast_possible_truncation)]
const fn compute_header_size(content_size: usize) -> usize {
    let mut header_size = content_size + 1;
    loop {
        let needed = varint_len_for_header_size(header_size) + content_size;
        if needed <= header_size {
            return header_size;
        }
        header_size = needed;
    }
}

const fn varint_len_for_header_size(value: usize) -> usize {
    if value <= 0x7F {
        1
    } else if value <= 0x3FFF {
        2
    } else if value <= 0x001F_FFFF {
        3
    } else if value <= 0x0FFF_FFFF {
        4
    } else if value <= 0x07_FFFF_FFFF {
        5
    } else if value <= 0x03FF_FFFF_FFFF {
        6
    } else if value <= 0x01_FFFF_FFFF_FFFF {
        7
    } else if value <= 0xFF_FFFF_FFFF_FFFF {
        8
    } else {
        9
    }
}

#[cfg(all(target_arch = "x86_64", not(target_arch = "wasm32")))]
#[inline]
fn classify_integer_block(values: [i64; 4], use_avx2: bool) -> [IntegerEncoding; 4] {
    if use_avx2 {
        // SAFETY: guarded by runtime AVX2 feature detection.
        unsafe { classify_integer_block_avx2(values) }
    } else {
        values.map(scalar_integer_encoding)
    }
}

#[cfg(not(all(target_arch = "x86_64", not(target_arch = "wasm32"))))]
#[inline]
fn classify_integer_block(values: [i64; 4], _use_avx2: bool) -> [IntegerEncoding; 4] {
    values.map(scalar_integer_encoding)
}

#[cfg(all(target_arch = "x86_64", not(target_arch = "wasm32")))]
#[inline]
fn avx2_available() -> bool {
    std::arch::is_x86_feature_detected!("avx2")
}

#[cfg(not(all(target_arch = "x86_64", not(target_arch = "wasm32"))))]
#[inline]
const fn avx2_available() -> bool {
    false
}

#[cfg(all(target_arch = "x86_64", not(target_arch = "wasm32")))]
#[target_feature(enable = "avx2")]
unsafe fn classify_integer_block_avx2(values: [i64; 4]) -> [IntegerEncoding; 4] {
    use std::arch::x86_64::{
        __m256i, _mm256_cmpeq_epi64, _mm256_cmpgt_epi64, _mm256_loadu_si256, _mm256_set1_epi64x,
        _mm256_setzero_si256, _mm256_storeu_si256, _mm256_xor_si256,
    };

    let values_vec = unsafe { _mm256_loadu_si256(values.as_ptr().cast::<__m256i>()) };
    let zero = _mm256_setzero_si256();
    let sign_mask = _mm256_cmpgt_epi64(zero, values_vec);
    let normalized = _mm256_xor_si256(values_vec, sign_mask);

    let eq_zero = _mm256_cmpeq_epi64(values_vec, zero);
    let eq_one = _mm256_cmpeq_epi64(values_vec, _mm256_set1_epi64x(1));
    let gt_127 = _mm256_cmpgt_epi64(normalized, _mm256_set1_epi64x(127));
    let gt_32k = _mm256_cmpgt_epi64(normalized, _mm256_set1_epi64x(32_767));
    let gt_8m = _mm256_cmpgt_epi64(normalized, _mm256_set1_epi64x(8_388_607));
    let gt_2g = _mm256_cmpgt_epi64(normalized, _mm256_set1_epi64x(2_147_483_647));
    let gt_48b = _mm256_cmpgt_epi64(normalized, _mm256_set1_epi64x(0x0000_7FFF_FFFF_FFFF));

    let mut eq_zero_lanes = [0_i64; 4];
    let mut eq_one_lanes = [0_i64; 4];
    let mut gt_127_lanes = [0_i64; 4];
    let mut gt_32k_lanes = [0_i64; 4];
    let mut gt_8m_lanes = [0_i64; 4];
    let mut gt_2g_lanes = [0_i64; 4];
    let mut gt_48b_lanes = [0_i64; 4];

    unsafe {
        _mm256_storeu_si256(eq_zero_lanes.as_mut_ptr().cast::<__m256i>(), eq_zero);
        _mm256_storeu_si256(eq_one_lanes.as_mut_ptr().cast::<__m256i>(), eq_one);
        _mm256_storeu_si256(gt_127_lanes.as_mut_ptr().cast::<__m256i>(), gt_127);
        _mm256_storeu_si256(gt_32k_lanes.as_mut_ptr().cast::<__m256i>(), gt_32k);
        _mm256_storeu_si256(gt_8m_lanes.as_mut_ptr().cast::<__m256i>(), gt_8m);
        _mm256_storeu_si256(gt_2g_lanes.as_mut_ptr().cast::<__m256i>(), gt_2g);
        _mm256_storeu_si256(gt_48b_lanes.as_mut_ptr().cast::<__m256i>(), gt_48b);
    }

    std::array::from_fn(|idx| {
        let serial_type = if eq_zero_lanes[idx] != 0 {
            8
        } else if eq_one_lanes[idx] != 0 {
            9
        } else if gt_127_lanes[idx] == 0 {
            1
        } else if gt_32k_lanes[idx] == 0 {
            2
        } else if gt_8m_lanes[idx] == 0 {
            3
        } else if gt_2g_lanes[idx] == 0 {
            4
        } else if gt_48b_lanes[idx] == 0 {
            5
        } else {
            6
        };
        IntegerEncoding::from_serial_type(serial_type)
    })
}

#[inline]
fn write_integer_payload(value: i64, payload_len: usize, dst: &mut [u8]) {
    if payload_len == 0 {
        return;
    }
    let bytes = value.to_be_bytes();
    dst.copy_from_slice(&bytes[8 - payload_len..]);
}

#[inline]
fn write_classified_block(
    values: &[i64],
    layouts: &[IntegerEncoding],
    buf: &mut [u8],
    header_offset: &mut usize,
    body_offset: &mut usize,
) {
    for (value, layout) in values.iter().zip(layouts.iter()) {
        buf[*header_offset] = layout.serial_type;
        *header_offset += 1;

        let payload_len = usize::from(layout.payload_len);
        let body_end = *body_offset + payload_len;
        write_integer_payload(*value, payload_len, &mut buf[*body_offset..body_end]);
        *body_offset = body_end;
    }
}

pub(crate) fn try_serialize_integer_record_iter_into<'a, I>(values: I, buf: &mut Vec<u8>) -> bool
where
    I: Iterator<Item = &'a SqliteValue> + Clone,
{
    let use_avx2 = avx2_available();
    let mut body_size = 0usize;
    let mut column_count = 0usize;
    let mut block_values = [0_i64; 4];
    let mut block_len = 0usize;

    for value in values.clone() {
        let SqliteValue::Integer(integer) = value else {
            return false;
        };

        block_values[block_len] = *integer;
        block_len += 1;
        column_count += 1;

        if block_len == 4 {
            let layouts = classify_integer_block(block_values, use_avx2);
            body_size += layouts
                .iter()
                .map(|layout| usize::from(layout.payload_len))
                .sum::<usize>();
            block_len = 0;
        }
    }

    for value in block_values.iter().take(block_len) {
        body_size += usize::from(scalar_integer_encoding(*value).payload_len);
    }

    let header_size = compute_header_size(column_count);
    let total_size = header_size + body_size;
    buf.clear();
    buf.resize(total_size, 0);

    let mut header_offset = write_varint(
        buf.as_mut_slice(),
        u64::try_from(header_size).unwrap_or(u64::MAX),
    );
    let mut body_offset = header_size;
    let mut encode_block_values = [0_i64; 4];
    let mut encode_block_len = 0usize;

    for value in values {
        let SqliteValue::Integer(integer) = value else {
            return false;
        };

        encode_block_values[encode_block_len] = *integer;
        encode_block_len += 1;

        if encode_block_len == 4 {
            let layouts = classify_integer_block(encode_block_values, use_avx2);
            write_classified_block(
                &encode_block_values,
                &layouts,
                buf.as_mut_slice(),
                &mut header_offset,
                &mut body_offset,
            );
            encode_block_len = 0;
        }
    }

    if encode_block_len > 0 {
        // Keep the tail path explicit rather than using `array::from_fn` here.
        // Nightly type inference around the const-generic array length has
        // been flaky on remote workers, and the fixed four-lane shape is small.
        let mut layouts = [IntegerEncoding::default(); 4];
        for idx in 0..encode_block_len {
            layouts[idx] = scalar_integer_encoding(encode_block_values[idx]);
        }
        write_classified_block(
            &encode_block_values[..encode_block_len],
            &layouts[..encode_block_len],
            buf.as_mut_slice(),
            &mut header_offset,
            &mut body_offset,
        );
    }

    debug_assert_eq!(header_offset, header_size);
    debug_assert_eq!(body_offset, total_size);
    true
}

#[cfg(test)]
mod tests {
    use super::try_serialize_integer_record_iter_into;
    use fsqlite_types::record::serialize_record;
    use fsqlite_types::value::SqliteValue;

    #[test]
    fn integer_record_fast_path_matches_scalar_record_bytes() {
        let row = vec![
            SqliteValue::Integer(0),
            SqliteValue::Integer(1),
            SqliteValue::Integer(-128),
            SqliteValue::Integer(32_767),
            SqliteValue::Integer(32_768),
            SqliteValue::Integer(8_388_607),
            SqliteValue::Integer(8_388_608),
            SqliteValue::Integer(i64::MIN),
        ];

        let mut fast = Vec::new();
        assert!(try_serialize_integer_record_iter_into(
            row.iter(),
            &mut fast
        ));
        assert_eq!(fast, serialize_record(&row));
    }

    #[test]
    fn integer_record_fast_path_rejects_non_integer_rows() {
        let row = vec![
            SqliteValue::Integer(7),
            SqliteValue::Text("not-an-integer".into()),
            SqliteValue::Integer(9),
        ];

        let mut fast = Vec::from([0xAA, 0xBB]);
        assert!(!try_serialize_integer_record_iter_into(
            row.iter(),
            &mut fast
        ));
        assert_eq!(fast, vec![0xAA, 0xBB]);
    }

    #[test]
    fn integer_record_fast_path_handles_large_headers() {
        let row = (0_i64..140).map(SqliteValue::Integer).collect::<Vec<_>>();

        let mut fast = Vec::new();
        assert!(try_serialize_integer_record_iter_into(
            row.iter(),
            &mut fast
        ));
        assert_eq!(fast, serialize_record(&row));
    }
}
