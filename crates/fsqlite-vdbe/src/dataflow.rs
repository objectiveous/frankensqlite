//! Weighted-row dataflow substrate for Bloodstream VDBE automata.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use fsqlite_types::SqliteValue;

/// A SQLite row paired with its algebraic multiplicity in a differential stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedRow {
    /// Row payload.
    pub values: Vec<SqliteValue>,
    /// Algebraic multiplicity. Inserts are positive, deletes are negative, and
    /// consolidated deltas can carry larger magnitudes.
    pub weight: i64,
}

impl WeightedRow {
    /// Construct a row with an explicit algebraic weight.
    pub fn new(values: Vec<SqliteValue>, weight: i64) -> Self {
        Self { values, weight }
    }

    /// Construct an inserted row (`+1`).
    pub fn insert(values: Vec<SqliteValue>) -> Self {
        Self::new(values, 1)
    }

    /// Construct a deleted row (`-1`).
    pub fn delete(values: Vec<SqliteValue>) -> Self {
        Self::new(values, -1)
    }

    /// Number of values in the row payload.
    pub fn width(&self) -> usize {
        self.values.len()
    }

    /// Whether this delta has no effect and can be elided.
    pub fn is_zero(&self) -> bool {
        self.weight == 0
    }

    fn project(&self, columns: &[usize]) -> DataflowResult<Vec<SqliteValue>> {
        columns
            .iter()
            .map(|&column| {
                self.values
                    .get(column)
                    .cloned()
                    .ok_or(DataflowError::ColumnOutOfBounds {
                        column,
                        width: self.values.len(),
                    })
            })
            .collect()
    }
}

/// Errors surfaced by the weighted-row dataflow automaton.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataflowError {
    /// A requested column index exceeded the row width.
    ColumnOutOfBounds { column: usize, width: usize },
    /// Join key mappings must specify the same number of left and right columns.
    JoinKeyArityMismatch { left: usize, right: usize },
    /// Input no longer matches the schema width captured when the automaton was built.
    SchemaChanged {
        expected_width: usize,
        actual_width: usize,
    },
}

impl fmt::Display for DataflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColumnOutOfBounds { column, width } => {
                write!(f, "column {column} out of bounds for row width {width}")
            }
            Self::JoinKeyArityMismatch { left, right } => {
                write!(f, "join key arity mismatch: left={left}, right={right}")
            }
            Self::SchemaChanged {
                expected_width,
                actual_width,
            } => write!(
                f,
                "schema changed: expected row width {expected_width}, got {actual_width}"
            ),
        }
    }
}

impl Error for DataflowError {}

type DataflowResult<T> = std::result::Result<T, DataflowError>;

/// Ordered operator sequence for a bounded differential VDBE automaton.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataflowAutomaton {
    expected_input_width: Option<usize>,
    operators: Vec<DataflowOperator>,
}

impl DataflowAutomaton {
    /// Create an automaton with no captured input schema.
    pub fn new(operators: Vec<DataflowOperator>) -> Self {
        Self {
            expected_input_width: None,
            operators,
        }
    }

    /// Create an automaton that fail-closes when input row width changes.
    pub fn with_input_width(expected_input_width: usize, operators: Vec<DataflowOperator>) -> Self {
        Self {
            expected_input_width: Some(expected_input_width),
            operators,
        }
    }

    /// Execute the operator sequence over a weighted-row batch.
    pub fn execute(&self, rows: &[WeightedRow]) -> DataflowResult<Vec<WeightedRow>> {
        tracing::debug!(
            target: "fsqlite_vdbe::dataflow",
            event = "execute",
            operators = self.operators.len(),
            input_rows = rows.len()
        );

        let mut current = self.validate_and_elide_zero_rows(rows)?;
        for operator in &self.operators {
            current = operator.apply(&current)?;
        }

        tracing::debug!(
            target: "fsqlite_vdbe::dataflow",
            event = "execute_complete",
            operators = self.operators.len(),
            output_rows = current.len()
        );
        Ok(current)
    }

    /// Access the operator list.
    pub fn operators(&self) -> &[DataflowOperator] {
        &self.operators
    }

    fn validate_and_elide_zero_rows(
        &self,
        rows: &[WeightedRow],
    ) -> DataflowResult<Vec<WeightedRow>> {
        let mut current = Vec::with_capacity(rows.len());
        for row in rows {
            if row.is_zero() {
                continue;
            }
            if let Some(expected_width) = self.expected_input_width
                && row.width() != expected_width
            {
                return Err(DataflowError::SchemaChanged {
                    expected_width,
                    actual_width: row.width(),
                });
            }
            current.push(row.clone());
        }
        Ok(current)
    }
}

/// Primitive weighted-row operators that preserve algebraic multiplicity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataflowOperator {
    /// Keep only rows where `column == value`.
    FilterEq { column: usize, value: SqliteValue },
    /// Keep only the requested columns, preserving row weight.
    Project { columns: Vec<usize> },
    /// Consolidate algebraic weights by key and elide zero-weight results.
    ConsolidateByKey { key_columns: Vec<usize> },
    /// Consolidate algebraic weights by complete row value.
    ConsolidateRows,
    /// Multiply every row weight by `factor`, eliding zero-weight output rows.
    ScaleWeight { factor: i64 },
    /// Compute `DeltaLeft JOIN Right`, with the current stream as the delta-left input.
    DeltaJoinLeft {
        stable_right: Vec<WeightedRow>,
        key_spec: JoinKeySpec,
    },
    /// Compute `Left JOIN DeltaRight`, with the current stream as the delta-right input.
    DeltaJoinRight {
        stable_left: Vec<WeightedRow>,
        key_spec: JoinKeySpec,
    },
    /// Compute the full join delta for simultaneous left and right relation changes.
    DeltaJoinUpdate {
        stable_left: Vec<WeightedRow>,
        stable_right: Vec<WeightedRow>,
        delta_right: Vec<WeightedRow>,
        key_spec: JoinKeySpec,
    },
}

impl DataflowOperator {
    fn apply(&self, rows: &[WeightedRow]) -> DataflowResult<Vec<WeightedRow>> {
        match self {
            Self::FilterEq { column, value } => rows
                .iter()
                .filter_map(|row| match row.values.get(*column) {
                    Some(candidate) if candidate == value => Some(Ok(row.clone())),
                    Some(_) => None,
                    None => Some(Err(DataflowError::ColumnOutOfBounds {
                        column: *column,
                        width: row.width(),
                    })),
                })
                .collect(),
            Self::Project { columns } => rows
                .iter()
                .map(|row| Ok(WeightedRow::new(row.project(columns)?, row.weight)))
                .collect(),
            Self::ConsolidateByKey { key_columns } => consolidate_by_key(rows, key_columns),
            Self::ConsolidateRows => Ok(consolidate_rows(rows.iter().cloned().collect())),
            Self::ScaleWeight { factor } => Ok(scale_weights(rows, *factor)),
            Self::DeltaJoinLeft {
                stable_right,
                key_spec,
            } => delta_join_left(rows, stable_right, key_spec),
            Self::DeltaJoinRight {
                stable_left,
                key_spec,
            } => delta_join_right(stable_left, rows, key_spec),
            Self::DeltaJoinUpdate {
                stable_left,
                stable_right,
                delta_right,
                key_spec,
            } => delta_join_update(stable_left, rows, stable_right, delta_right, key_spec),
        }
    }
}

/// Column mapping for delta-aware inner joins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinKeySpec {
    /// Key columns read from the left relation.
    pub left_columns: Vec<usize>,
    /// Key columns read from the right relation.
    pub right_columns: Vec<usize>,
}

impl JoinKeySpec {
    /// Construct a join-key spec.
    pub fn new(left_columns: Vec<usize>, right_columns: Vec<usize>) -> Self {
        Self {
            left_columns,
            right_columns,
        }
    }

    fn validate(&self) -> DataflowResult<()> {
        if self.left_columns.len() != self.right_columns.len() {
            return Err(DataflowError::JoinKeyArityMismatch {
                left: self.left_columns.len(),
                right: self.right_columns.len(),
            });
        }
        Ok(())
    }
}

fn consolidate_by_key(
    rows: &[WeightedRow],
    key_columns: &[usize],
) -> DataflowResult<Vec<WeightedRow>> {
    let mut groups: Vec<(Vec<SqliteValue>, i64)> = Vec::new();
    for row in rows {
        if row.is_zero() {
            continue;
        }
        let key = row.project(key_columns)?;
        if let Some((_, weight)) = groups.iter_mut().find(|(candidate, _)| *candidate == key) {
            *weight = weight.saturating_add(row.weight);
        } else {
            groups.push((key, row.weight));
        }
    }
    Ok(groups
        .into_iter()
        .filter_map(|(values, weight)| (weight != 0).then(|| WeightedRow::new(values, weight)))
        .collect())
}

fn index_weighted_rows<'a>(
    rows: &'a [WeightedRow],
    key_columns: &[usize],
) -> DataflowResult<BTreeMap<Vec<SqliteValue>, Vec<&'a WeightedRow>>> {
    let mut index: BTreeMap<Vec<SqliteValue>, Vec<&WeightedRow>> = BTreeMap::new();
    for row in rows {
        if row.is_zero() {
            continue;
        }
        let key = row.project(key_columns)?;
        index.entry(key).or_default().push(row);
    }
    Ok(index)
}

/// Consolidate identical output rows by algebraic weight, preserving first-seen order.
pub fn consolidate_rows(rows: Vec<WeightedRow>) -> Vec<WeightedRow> {
    let mut groups: Vec<(Vec<SqliteValue>, i64)> = Vec::new();
    for row in rows {
        if row.is_zero() {
            continue;
        }
        let values = row.values;
        if let Some((_, weight)) = groups
            .iter_mut()
            .find(|(candidate, _)| candidate == &values)
        {
            *weight = weight.saturating_add(row.weight);
        } else {
            groups.push((values, row.weight));
        }
    }
    groups
        .into_iter()
        .filter_map(|(values, weight)| (weight != 0).then(|| WeightedRow::new(values, weight)))
        .collect()
}

/// Multiply row weights by an algebraic factor, preserving row order and values.
pub fn scale_weights(rows: &[WeightedRow], factor: i64) -> Vec<WeightedRow> {
    if factor == 0 {
        return Vec::new();
    }

    rows.iter()
        .filter_map(|row| {
            let weight = row.weight.saturating_mul(factor);
            (weight != 0).then(|| WeightedRow::new(row.values.clone(), weight))
        })
        .collect()
}

/// Compute `DeltaLeft JOIN Right`, preserving algebraic weights.
pub fn delta_join_left(
    delta_left: &[WeightedRow],
    stable_right: &[WeightedRow],
    key_spec: &JoinKeySpec,
) -> DataflowResult<Vec<WeightedRow>> {
    key_spec.validate()?;
    let right_index = index_weighted_rows(stable_right, &key_spec.right_columns)?;
    let mut joined = Vec::new();

    for left in delta_left {
        if left.is_zero() {
            continue;
        }
        let left_key = left.project(&key_spec.left_columns)?;
        if let Some(matches) = right_index.get(&left_key) {
            for right in matches {
                let mut values = left.values.clone();
                values.extend(right.values.clone());
                joined.push(WeightedRow::new(
                    values,
                    left.weight.saturating_mul(right.weight),
                ));
            }
        }
    }

    tracing::debug!(
        target: "fsqlite_vdbe::dataflow",
        event = "delta_join_left",
        delta_rows = delta_left.len(),
        stable_rows = stable_right.len(),
        output_rows = joined.len()
    );
    Ok(joined)
}

/// Compute `Left JOIN DeltaRight`, preserving algebraic weights.
pub fn delta_join_right(
    stable_left: &[WeightedRow],
    delta_right: &[WeightedRow],
    key_spec: &JoinKeySpec,
) -> DataflowResult<Vec<WeightedRow>> {
    key_spec.validate()?;
    let left_index = index_weighted_rows(stable_left, &key_spec.left_columns)?;
    let mut joined = Vec::new();

    for right in delta_right {
        if right.is_zero() {
            continue;
        }
        let right_key = right.project(&key_spec.right_columns)?;
        if let Some(matches) = left_index.get(&right_key) {
            for left in matches {
                let mut values = left.values.clone();
                values.extend(right.values.clone());
                joined.push(WeightedRow::new(
                    values,
                    left.weight.saturating_mul(right.weight),
                ));
            }
        }
    }

    tracing::debug!(
        target: "fsqlite_vdbe::dataflow",
        event = "delta_join_right",
        stable_rows = stable_left.len(),
        delta_rows = delta_right.len(),
        output_rows = joined.len()
    );
    Ok(joined)
}

/// Compute the full join delta for simultaneous left and right relation changes.
///
/// Given old stable relations `L` and `R`, plus input deltas `dL` and `dR`,
/// this emits `(dL JOIN R) UNION (L JOIN dR) UNION (dL JOIN dR)` and
/// consolidates duplicate output rows by algebraic weight.
pub fn delta_join_update(
    stable_left: &[WeightedRow],
    delta_left: &[WeightedRow],
    stable_right: &[WeightedRow],
    delta_right: &[WeightedRow],
    key_spec: &JoinKeySpec,
) -> DataflowResult<Vec<WeightedRow>> {
    key_spec.validate()?;
    let mut joined = delta_join_left(delta_left, stable_right, key_spec)?;
    joined.extend(delta_join_right(stable_left, delta_right, key_spec)?);
    joined.extend(delta_join_left(delta_left, delta_right, key_spec)?);
    let joined = consolidate_rows(joined);

    tracing::debug!(
        target: "fsqlite_vdbe::dataflow",
        event = "delta_join_update",
        stable_left_rows = stable_left.len(),
        delta_left_rows = delta_left.len(),
        stable_right_rows = stable_right.len(),
        delta_right_rows = delta_right.len(),
        output_rows = joined.len()
    );
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::{
        DataflowAutomaton, DataflowError, DataflowOperator, JoinKeySpec, WeightedRow,
        delta_join_left, delta_join_right, delta_join_update, scale_weights,
    };
    use fsqlite_types::SqliteValue;

    fn int(value: i64) -> SqliteValue {
        SqliteValue::Integer(value)
    }

    #[test]
    fn automaton_filters_projects_and_preserves_weight() {
        let automaton = DataflowAutomaton::with_input_width(
            3,
            vec![
                DataflowOperator::FilterEq {
                    column: 1,
                    value: int(7),
                },
                DataflowOperator::Project {
                    columns: vec![2, 0],
                },
            ],
        );
        let rows = vec![
            WeightedRow::new(vec![int(1), int(7), int(10)], 3),
            WeightedRow::new(vec![int(2), int(8), int(20)], 5),
            WeightedRow::new(vec![int(3), int(7), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(actual, vec![WeightedRow::new(vec![int(10), int(1)], 3)]);
    }

    #[test]
    fn automaton_fail_closes_on_schema_width_change() {
        let automaton = DataflowAutomaton::with_input_width(2, Vec::new());

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), int(2), int(3)])])
            .expect_err("width mismatch should halt the automaton");

        assert_eq!(
            err,
            DataflowError::SchemaChanged {
                expected_width: 2,
                actual_width: 3
            }
        );
    }

    #[test]
    fn consolidate_by_key_preserves_first_key_order_and_elides_zeroes() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::ConsolidateByKey {
            key_columns: vec![0],
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(2), int(20)], 1),
            WeightedRow::new(vec![int(1), int(10)], 4),
            WeightedRow::new(vec![int(2), int(21)], -1),
            WeightedRow::new(vec![int(1), int(11)], 3),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![WeightedRow::new(vec![int(1)], 7)],
            "key 2 should cancel out and key 1 should retain first-seen ordering"
        );
    }

    #[test]
    fn consolidate_rows_operator_preserves_first_row_order_and_elides_zeroes() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::ConsolidateRows]);
        let rows = vec![
            WeightedRow::new(vec![int(2), int(20)], 1),
            WeightedRow::new(vec![int(1), int(10)], 3),
            WeightedRow::new(vec![int(2), int(20)], -1),
            WeightedRow::new(vec![int(1), int(10)], 4),
            WeightedRow::new(vec![int(3), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(actual, vec![WeightedRow::new(vec![int(1), int(10)], 7)]);
    }

    #[test]
    fn scale_weight_operator_inverts_weights_and_elides_zeroes() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::ScaleWeight { factor: -1 }]);
        let rows = vec![
            WeightedRow::new(vec![int(1), int(10)], 3),
            WeightedRow::new(vec![int(2), int(20)], -2),
            WeightedRow::new(vec![int(3), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::new(vec![int(1), int(10)], -3),
                WeightedRow::new(vec![int(2), int(20)], 2),
            ]
        );
    }

    #[test]
    fn scale_weights_zero_factor_elides_all_rows() {
        let rows = vec![
            WeightedRow::new(vec![int(1)], 3),
            WeightedRow::new(vec![int(2)], -2),
        ];

        let actual = scale_weights(&rows, 0);

        assert_eq!(actual, Vec::<WeightedRow>::new());
    }

    #[test]
    fn scale_weights_saturates_on_overflow() {
        let rows = vec![WeightedRow::new(vec![int(1)], i64::MAX)];

        let actual = scale_weights(&rows, 2);

        assert_eq!(actual, vec![WeightedRow::new(vec![int(1)], i64::MAX)]);
    }

    #[test]
    fn delta_join_left_multiplies_weights() {
        let spec = JoinKeySpec::new(vec![0], vec![0]);
        let delta_left = vec![
            WeightedRow::new(vec![int(1), int(10)], 2),
            WeightedRow::new(vec![int(2), int(20)], 5),
        ];
        let stable_right = vec![
            WeightedRow::new(vec![int(1), int(100)], -3),
            WeightedRow::new(vec![int(3), int(300)], 7),
        ];

        let actual =
            delta_join_left(&delta_left, &stable_right, &spec).expect("join should execute");

        assert_eq!(
            actual,
            vec![WeightedRow::new(
                vec![int(1), int(10), int(1), int(100)],
                -6
            )]
        );
    }

    #[test]
    fn delta_join_right_multiplies_weights() {
        let spec = JoinKeySpec::new(vec![0], vec![0]);
        let stable_left = vec![WeightedRow::new(vec![int(9), int(90)], 4)];
        let delta_right = vec![
            WeightedRow::new(vec![int(9), int(900)], -2),
            WeightedRow::new(vec![int(8), int(800)], -2),
        ];

        let actual =
            delta_join_right(&stable_left, &delta_right, &spec).expect("join should execute");

        assert_eq!(
            actual,
            vec![WeightedRow::new(
                vec![int(9), int(90), int(9), int(900)],
                -8
            )]
        );
    }

    #[test]
    fn join_key_arity_mismatch_is_rejected() {
        let spec = JoinKeySpec::new(vec![0, 1], vec![0]);

        let err = delta_join_left(&[], &[], &spec).expect_err("arity mismatch must fail");

        assert_eq!(
            err,
            DataflowError::JoinKeyArityMismatch { left: 2, right: 1 }
        );
    }

    #[test]
    fn delta_join_update_includes_both_sides_and_delta_delta_term() {
        let spec = JoinKeySpec::new(vec![0], vec![0]);
        let stable_left = vec![WeightedRow::new(vec![int(1), int(10)], 1)];
        let stable_right = vec![WeightedRow::new(vec![int(1), int(100)], 1)];
        let delta_left = vec![WeightedRow::new(vec![int(1), int(11)], 1)];
        let delta_right = vec![WeightedRow::new(vec![int(1), int(101)], 1)];

        let actual = delta_join_update(
            &stable_left,
            &delta_left,
            &stable_right,
            &delta_right,
            &spec,
        )
        .expect("join update should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::new(vec![int(1), int(11), int(1), int(100)], 1),
                WeightedRow::new(vec![int(1), int(10), int(1), int(101)], 1),
                WeightedRow::new(vec![int(1), int(11), int(1), int(101)], 1),
            ]
        );
    }

    #[test]
    fn delta_join_update_consolidates_duplicate_output_rows() {
        let spec = JoinKeySpec::new(vec![0], vec![0]);
        let stable_left = vec![WeightedRow::new(vec![int(1), int(10)], 1)];
        let stable_right = vec![WeightedRow::new(vec![int(1), int(100)], 1)];
        let delta_left = vec![WeightedRow::new(vec![int(1), int(10)], 1)];
        let delta_right = vec![WeightedRow::new(vec![int(1), int(100)], -1)];

        let actual = delta_join_update(
            &stable_left,
            &delta_left,
            &stable_right,
            &delta_right,
            &spec,
        )
        .expect("join update should execute");

        assert_eq!(
            actual,
            vec![WeightedRow::new(
                vec![int(1), int(10), int(1), int(100)],
                -1
            )]
        );
    }

    #[test]
    fn automaton_delta_join_update_uses_current_rows_as_left_delta() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::DeltaJoinUpdate {
            stable_left: vec![WeightedRow::new(vec![int(1), int(10)], 1)],
            stable_right: vec![WeightedRow::new(vec![int(1), int(100)], 1)],
            delta_right: vec![WeightedRow::new(vec![int(1), int(101)], 1)],
            key_spec: JoinKeySpec::new(vec![0], vec![0]),
        }]);

        let actual = automaton
            .execute(&[WeightedRow::new(vec![int(1), int(11)], 1)])
            .expect("join update operator should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::new(vec![int(1), int(11), int(1), int(100)], 1),
                WeightedRow::new(vec![int(1), int(10), int(1), int(101)], 1),
                WeightedRow::new(vec![int(1), int(11), int(1), int(101)], 1),
            ]
        );
    }
}
