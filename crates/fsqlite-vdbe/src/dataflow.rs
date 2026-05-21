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
    /// Integer predicate operators only accept integer value inputs.
    PredicateValueNotInteger { column: usize },
    /// Integer aggregate operators only accept integer value inputs.
    AggregateValueNotInteger { column: usize },
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
            Self::PredicateValueNotInteger { column } => {
                write!(f, "predicate value column {column} is not an integer")
            }
            Self::AggregateValueNotInteger { column } => {
                write!(f, "aggregate value column {column} is not an integer")
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
    /// Keep rows whose column value appears in the configured set.
    FilterInSet {
        column: usize,
        values: Vec<SqliteValue>,
    },
    /// Keep rows whose column value does not appear in the configured set.
    FilterNotInSet {
        column: usize,
        values: Vec<SqliteValue>,
    },
    /// Keep rows where an integer column lies in an inclusive range.
    FilterIntegerBetween {
        column: usize,
        lower: i64,
        upper: i64,
    },
    /// Keep rows where an integer column satisfies the comparison.
    FilterIntegerCompare {
        column: usize,
        op: IntegerComparison,
        value: i64,
    },
    /// Keep rows whose column nullness matches the requested predicate.
    FilterNull {
        column: usize,
        predicate: NullPredicate,
    },
    /// Keep only the requested columns, preserving row weight.
    Project { columns: Vec<usize> },
    /// Keep rows whose algebraic weight matches the requested sign.
    FilterWeightSign { sign: WeightSign },
    /// Append a constant payload value to each row, preserving row weight.
    AppendLiteral { value: SqliteValue },
    /// Consolidate algebraic weights by key and elide zero-weight results.
    ConsolidateByKey { key_columns: Vec<usize> },
    /// Emit one materialized `COUNT(*)` row per key.
    CountByKey { key_columns: Vec<usize> },
    /// Emit one materialized integer `SUM(value_column)` row per key.
    SumIntegerByKey {
        key_columns: Vec<usize>,
        value_column: usize,
    },
    /// Emit one materialized integer `MIN(value_column)` row per key.
    MinIntegerByKey {
        key_columns: Vec<usize>,
        value_column: usize,
    },
    /// Emit one materialized integer `MAX(value_column)` row per key.
    MaxIntegerByKey {
        key_columns: Vec<usize>,
        value_column: usize,
    },
    /// Emit one materialized floating-point `AVG(value_column)` row per key.
    AverageIntegerByKey {
        key_columns: Vec<usize>,
        value_column: usize,
    },
    /// Consolidate algebraic weights by complete row value.
    ConsolidateRows,
    /// Multiply every row weight by `factor`, eliding zero-weight output rows.
    ScaleWeight { factor: i64 },
    /// Rewrite every non-zero input row to a fixed output weight.
    SetWeight { weight: i64 },
    /// Keep rows with positive accumulated weight as set membership.
    ThresholdPositive,
    /// Append each row's algebraic weight as an integer payload column for emission.
    AppendWeightColumn,
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
            Self::FilterInSet { column, values } => filter_in_set(rows, *column, values),
            Self::FilterNotInSet { column, values } => filter_not_in_set(rows, *column, values),
            Self::FilterIntegerBetween {
                column,
                lower,
                upper,
            } => filter_integer_between(rows, *column, *lower, *upper),
            Self::FilterIntegerCompare { column, op, value } => {
                filter_integer_compare(rows, *column, *op, *value)
            }
            Self::FilterNull { column, predicate } => filter_null(rows, *column, *predicate),
            Self::Project { columns } => rows
                .iter()
                .map(|row| Ok(WeightedRow::new(row.project(columns)?, row.weight)))
                .collect(),
            Self::FilterWeightSign { sign } => Ok(filter_weight_sign(rows, *sign)),
            Self::AppendLiteral { value } => Ok(append_literal_column(rows, value)),
            Self::ConsolidateByKey { key_columns } => consolidate_by_key(rows, key_columns),
            Self::CountByKey { key_columns } => count_by_key(rows, key_columns),
            Self::SumIntegerByKey {
                key_columns,
                value_column,
            } => sum_integer_by_key(rows, key_columns, *value_column),
            Self::MinIntegerByKey {
                key_columns,
                value_column,
            } => min_integer_by_key(rows, key_columns, *value_column),
            Self::MaxIntegerByKey {
                key_columns,
                value_column,
            } => max_integer_by_key(rows, key_columns, *value_column),
            Self::AverageIntegerByKey {
                key_columns,
                value_column,
            } => average_integer_by_key(rows, key_columns, *value_column),
            Self::ConsolidateRows => Ok(consolidate_rows(rows.iter().cloned().collect())),
            Self::ScaleWeight { factor } => Ok(scale_weights(rows, *factor)),
            Self::SetWeight { weight } => Ok(set_weights(rows, *weight)),
            Self::ThresholdPositive => Ok(threshold_positive(rows)),
            Self::AppendWeightColumn => Ok(append_weight_column(rows)),
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

/// Integer predicate comparison used by dataflow filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerComparison {
    /// Match values strictly below the threshold.
    LessThan,
    /// Match values at or below the threshold.
    LessOrEqual,
    /// Match values strictly above the threshold.
    GreaterThan,
    /// Match values at or above the threshold.
    GreaterOrEqual,
}

impl IntegerComparison {
    fn matches(self, candidate: i64, threshold: i64) -> bool {
        match self {
            Self::LessThan => candidate < threshold,
            Self::LessOrEqual => candidate <= threshold,
            Self::GreaterThan => candidate > threshold,
            Self::GreaterOrEqual => candidate >= threshold,
        }
    }
}

/// Null predicate used by dataflow filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullPredicate {
    /// Match `NULL` values.
    IsNull,
    /// Match non-`NULL` values.
    IsNotNull,
}

impl NullPredicate {
    fn matches(self, value: &SqliteValue) -> bool {
        match self {
            Self::IsNull => matches!(value, SqliteValue::Null),
            Self::IsNotNull => !matches!(value, SqliteValue::Null),
        }
    }
}

/// Algebraic weight sign used to split insert and delete delta streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightSign {
    /// Positive-weight rows, commonly insertion deltas.
    Positive,
    /// Negative-weight rows, commonly deletion deltas.
    Negative,
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

fn filter_weight_sign(rows: &[WeightedRow], sign: WeightSign) -> Vec<WeightedRow> {
    rows.iter()
        .filter_map(|row| match (sign, row.weight) {
            (WeightSign::Positive, weight) if weight > 0 => Some(row.clone()),
            (WeightSign::Negative, weight) if weight < 0 => Some(row.clone()),
            _ => None,
        })
        .collect()
}

fn append_literal_column(rows: &[WeightedRow], value: &SqliteValue) -> Vec<WeightedRow> {
    rows.iter()
        .filter_map(|row| {
            if row.is_zero() {
                return None;
            }
            let mut values = row.values.clone();
            values.push(value.clone());
            Some(WeightedRow::new(values, row.weight))
        })
        .collect()
}

fn filter_in_set(
    rows: &[WeightedRow],
    column: usize,
    values: &[SqliteValue],
) -> DataflowResult<Vec<WeightedRow>> {
    rows.iter()
        .filter_map(|row| match row.values.get(column) {
            Some(candidate) if values.iter().any(|value| value == candidate) => {
                Some(Ok(row.clone()))
            }
            Some(_) => None,
            None => Some(Err(DataflowError::ColumnOutOfBounds {
                column,
                width: row.width(),
            })),
        })
        .collect()
}

fn filter_not_in_set(
    rows: &[WeightedRow],
    column: usize,
    values: &[SqliteValue],
) -> DataflowResult<Vec<WeightedRow>> {
    rows.iter()
        .filter_map(|row| match row.values.get(column) {
            Some(candidate) if values.iter().all(|value| value != candidate) => {
                Some(Ok(row.clone()))
            }
            Some(_) => None,
            None => Some(Err(DataflowError::ColumnOutOfBounds {
                column,
                width: row.width(),
            })),
        })
        .collect()
}

fn filter_integer_between(
    rows: &[WeightedRow],
    column: usize,
    lower: i64,
    upper: i64,
) -> DataflowResult<Vec<WeightedRow>> {
    rows.iter()
        .filter_map(|row| match row.values.get(column) {
            Some(SqliteValue::Integer(candidate)) if lower <= *candidate && *candidate <= upper => {
                Some(Ok(row.clone()))
            }
            Some(SqliteValue::Integer(_)) => None,
            Some(_) => Some(Err(DataflowError::PredicateValueNotInteger { column })),
            None => Some(Err(DataflowError::ColumnOutOfBounds {
                column,
                width: row.width(),
            })),
        })
        .collect()
}

fn filter_integer_compare(
    rows: &[WeightedRow],
    column: usize,
    op: IntegerComparison,
    value: i64,
) -> DataflowResult<Vec<WeightedRow>> {
    rows.iter()
        .filter_map(|row| match row.values.get(column) {
            Some(SqliteValue::Integer(candidate)) if op.matches(*candidate, value) => {
                Some(Ok(row.clone()))
            }
            Some(SqliteValue::Integer(_)) => None,
            Some(_) => Some(Err(DataflowError::PredicateValueNotInteger { column })),
            None => Some(Err(DataflowError::ColumnOutOfBounds {
                column,
                width: row.width(),
            })),
        })
        .collect()
}

fn filter_null(
    rows: &[WeightedRow],
    column: usize,
    predicate: NullPredicate,
) -> DataflowResult<Vec<WeightedRow>> {
    rows.iter()
        .filter_map(|row| match row.values.get(column) {
            Some(value) if predicate.matches(value) => Some(Ok(row.clone())),
            Some(_) => None,
            None => Some(Err(DataflowError::ColumnOutOfBounds {
                column,
                width: row.width(),
            })),
        })
        .collect()
}

fn count_by_key(rows: &[WeightedRow], key_columns: &[usize]) -> DataflowResult<Vec<WeightedRow>> {
    let mut groups: Vec<(Vec<SqliteValue>, i64)> = Vec::new();
    for row in rows {
        if row.is_zero() {
            continue;
        }
        let key = row.project(key_columns)?;
        if let Some((_, count)) = groups.iter_mut().find(|(candidate, _)| *candidate == key) {
            *count = count.saturating_add(row.weight);
        } else {
            groups.push((key, row.weight));
        }
    }

    Ok(groups
        .into_iter()
        .filter_map(|(mut values, count)| {
            if count == 0 {
                return None;
            }
            values.push(SqliteValue::Integer(count));
            Some(WeightedRow::insert(values))
        })
        .collect())
}

fn sum_integer_by_key(
    rows: &[WeightedRow],
    key_columns: &[usize],
    value_column: usize,
) -> DataflowResult<Vec<WeightedRow>> {
    let mut groups: Vec<(Vec<SqliteValue>, i64)> = Vec::new();
    for row in rows {
        if row.is_zero() {
            continue;
        }
        let key = row.project(key_columns)?;
        let value = integer_value_at(row, value_column)?;
        let weighted_value = value.saturating_mul(row.weight);
        if let Some((_, sum)) = groups.iter_mut().find(|(candidate, _)| *candidate == key) {
            *sum = sum.saturating_add(weighted_value);
        } else {
            groups.push((key, weighted_value));
        }
    }

    Ok(groups
        .into_iter()
        .filter_map(|(mut values, sum)| {
            if sum == 0 {
                return None;
            }
            values.push(SqliteValue::Integer(sum));
            Some(WeightedRow::insert(values))
        })
        .collect())
}

fn min_integer_by_key(
    rows: &[WeightedRow],
    key_columns: &[usize],
    value_column: usize,
) -> DataflowResult<Vec<WeightedRow>> {
    let mut groups: Vec<(Vec<SqliteValue>, i64)> = Vec::new();
    for row in rows {
        if row.weight <= 0 {
            continue;
        }
        let key = row.project(key_columns)?;
        let value = integer_value_at(row, value_column)?;
        if let Some((_, min_value)) = groups.iter_mut().find(|(candidate, _)| *candidate == key) {
            *min_value = (*min_value).min(value);
        } else {
            groups.push((key, value));
        }
    }

    Ok(groups
        .into_iter()
        .map(|(mut values, min_value)| {
            values.push(SqliteValue::Integer(min_value));
            WeightedRow::insert(values)
        })
        .collect())
}

fn max_integer_by_key(
    rows: &[WeightedRow],
    key_columns: &[usize],
    value_column: usize,
) -> DataflowResult<Vec<WeightedRow>> {
    let mut groups: Vec<(Vec<SqliteValue>, i64)> = Vec::new();
    for row in rows {
        if row.weight <= 0 {
            continue;
        }
        let key = row.project(key_columns)?;
        let value = integer_value_at(row, value_column)?;
        if let Some((_, max_value)) = groups.iter_mut().find(|(candidate, _)| *candidate == key) {
            *max_value = (*max_value).max(value);
        } else {
            groups.push((key, value));
        }
    }

    Ok(groups
        .into_iter()
        .map(|(mut values, max_value)| {
            values.push(SqliteValue::Integer(max_value));
            WeightedRow::insert(values)
        })
        .collect())
}

fn average_integer_by_key(
    rows: &[WeightedRow],
    key_columns: &[usize],
    value_column: usize,
) -> DataflowResult<Vec<WeightedRow>> {
    let mut groups: Vec<(Vec<SqliteValue>, i64, i64)> = Vec::new();
    for row in rows {
        if row.weight <= 0 {
            continue;
        }
        let key = row.project(key_columns)?;
        let value = integer_value_at(row, value_column)?;
        let weighted_value = value.saturating_mul(row.weight);
        if let Some((_, sum, count)) = groups
            .iter_mut()
            .find(|(candidate, _, _)| *candidate == key)
        {
            *sum = sum.saturating_add(weighted_value);
            *count = count.saturating_add(row.weight);
        } else {
            groups.push((key, weighted_value, row.weight));
        }
    }

    Ok(groups
        .into_iter()
        .filter_map(|(mut values, sum, count)| {
            if count <= 0 {
                return None;
            }
            values.push(SqliteValue::Float(sum as f64 / count as f64));
            Some(WeightedRow::insert(values))
        })
        .collect())
}

fn integer_value_at(row: &WeightedRow, value_column: usize) -> DataflowResult<i64> {
    match row.values.get(value_column) {
        Some(SqliteValue::Integer(value)) => Ok(*value),
        Some(_) => Err(DataflowError::AggregateValueNotInteger {
            column: value_column,
        }),
        None => Err(DataflowError::ColumnOutOfBounds {
            column: value_column,
            width: row.width(),
        }),
    }
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

/// Rewrite non-zero input rows to a fixed algebraic weight.
pub fn set_weights(rows: &[WeightedRow], weight: i64) -> Vec<WeightedRow> {
    if weight == 0 {
        return Vec::new();
    }

    rows.iter()
        .filter_map(|row| (!row.is_zero()).then(|| WeightedRow::new(row.values.clone(), weight)))
        .collect()
}

/// Convert accumulated differential counts into positive set membership.
pub fn threshold_positive(rows: &[WeightedRow]) -> Vec<WeightedRow> {
    consolidate_rows(rows.to_vec())
        .into_iter()
        .filter_map(|row| (row.weight > 0).then(|| WeightedRow::insert(row.values)))
        .collect()
}

/// Materialize algebraic weights into payload rows for downstream delta emission.
pub fn append_weight_column(rows: &[WeightedRow]) -> Vec<WeightedRow> {
    rows.iter()
        .filter_map(|row| {
            if row.is_zero() {
                return None;
            }
            let mut values = row.values.clone();
            values.push(SqliteValue::Integer(row.weight));
            Some(WeightedRow::insert(values))
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
        DataflowAutomaton, DataflowError, DataflowOperator, IntegerComparison, JoinKeySpec,
        NullPredicate, WeightSign, WeightedRow, delta_join_left, delta_join_right,
        delta_join_update, scale_weights, set_weights, threshold_positive,
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
    fn filter_in_set_keeps_matching_weighted_rows() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterInSet {
            column: 1,
            values: vec![int(3), int(5)],
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(1), int(2)], 7),
            WeightedRow::new(vec![int(2), int(3)], -2),
            WeightedRow::new(vec![int(3), int(5)], 4),
            WeightedRow::new(vec![int(4), int(5)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::new(vec![int(2), int(3)], -2),
                WeightedRow::new(vec![int(3), int(5)], 4),
            ]
        );
    }

    #[test]
    fn filter_in_set_rejects_out_of_bounds_columns() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterInSet {
            column: 2,
            values: vec![int(3)],
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), int(2)])])
            .expect_err("invalid in-set predicate column should fail");

        assert_eq!(
            err,
            DataflowError::ColumnOutOfBounds {
                column: 2,
                width: 2
            }
        );
    }

    #[test]
    fn filter_not_in_set_keeps_non_matching_weighted_rows() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterNotInSet {
            column: 1,
            values: vec![int(3), int(5)],
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(1), int(2)], 7),
            WeightedRow::new(vec![int(2), int(3)], -2),
            WeightedRow::new(vec![int(3), int(5)], 4),
            WeightedRow::new(vec![int(4), int(8)], -6),
            WeightedRow::new(vec![int(5), int(8)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::new(vec![int(1), int(2)], 7),
                WeightedRow::new(vec![int(4), int(8)], -6),
            ]
        );
    }

    #[test]
    fn filter_not_in_set_empty_values_keeps_all_non_zero_rows() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterNotInSet {
            column: 1,
            values: Vec::new(),
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(1), int(2)], 7),
            WeightedRow::new(vec![int(2), int(3)], -2),
            WeightedRow::new(vec![int(3), int(5)], 0),
        ];

        assert_eq!(
            automaton.execute(&rows).expect("dataflow should execute"),
            vec![
                WeightedRow::new(vec![int(1), int(2)], 7),
                WeightedRow::new(vec![int(2), int(3)], -2),
            ]
        );
    }

    #[test]
    fn filter_not_in_set_rejects_out_of_bounds_columns() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterNotInSet {
            column: 2,
            values: vec![int(3)],
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), int(2)])])
            .expect_err("invalid not-in-set predicate column should fail");

        assert_eq!(
            err,
            DataflowError::ColumnOutOfBounds {
                column: 2,
                width: 2
            }
        );
    }

    #[test]
    fn filter_integer_between_keeps_matching_weighted_rows() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterIntegerBetween {
            column: 1,
            lower: 10,
            upper: 20,
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(1), int(9)], 3),
            WeightedRow::new(vec![int(2), int(10)], -2),
            WeightedRow::new(vec![int(3), int(15)], 4),
            WeightedRow::new(vec![int(4), int(20)], 5),
            WeightedRow::new(vec![int(5), int(21)], 6),
            WeightedRow::new(vec![int(6), int(15)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::new(vec![int(2), int(10)], -2),
                WeightedRow::new(vec![int(3), int(15)], 4),
                WeightedRow::new(vec![int(4), int(20)], 5),
            ]
        );
    }

    #[test]
    fn filter_integer_between_inverted_range_matches_no_rows() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterIntegerBetween {
            column: 1,
            lower: 20,
            upper: 10,
        }]);
        let rows = vec![WeightedRow::new(vec![int(1), int(15)], 3)];

        assert_eq!(
            automaton.execute(&rows).expect("dataflow should execute"),
            Vec::<WeightedRow>::new()
        );
    }

    #[test]
    fn filter_integer_between_rejects_non_integer_values() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterIntegerBetween {
            column: 1,
            lower: 10,
            upper: 20,
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![
                int(1),
                SqliteValue::Text("15".into()),
            ])])
            .expect_err("non-integer predicate input should fail");

        assert_eq!(err, DataflowError::PredicateValueNotInteger { column: 1 });
    }

    #[test]
    fn filter_integer_between_rejects_out_of_bounds_columns() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterIntegerBetween {
            column: 2,
            lower: 10,
            upper: 20,
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), int(2)])])
            .expect_err("invalid between predicate column should fail");

        assert_eq!(
            err,
            DataflowError::ColumnOutOfBounds {
                column: 2,
                width: 2
            }
        );
    }

    #[test]
    fn filter_integer_compare_keeps_matching_weighted_rows() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterIntegerCompare {
            column: 1,
            op: IntegerComparison::GreaterOrEqual,
            value: 10,
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(1), int(9)], 3),
            WeightedRow::new(vec![int(2), int(10)], -2),
            WeightedRow::new(vec![int(3), int(11)], 4),
            WeightedRow::new(vec![int(4), int(12)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::new(vec![int(2), int(10)], -2),
                WeightedRow::new(vec![int(3), int(11)], 4),
            ]
        );
    }

    #[test]
    fn filter_integer_compare_rejects_non_integer_values() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterIntegerCompare {
            column: 1,
            op: IntegerComparison::LessThan,
            value: 10,
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), SqliteValue::Float(1.5)])])
            .expect_err("non-integer predicate input should fail");

        assert_eq!(err, DataflowError::PredicateValueNotInteger { column: 1 });
    }

    #[test]
    fn filter_integer_compare_rejects_out_of_bounds_columns() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterIntegerCompare {
            column: 2,
            op: IntegerComparison::LessOrEqual,
            value: 10,
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), int(2)])])
            .expect_err("invalid predicate column should fail");

        assert_eq!(
            err,
            DataflowError::ColumnOutOfBounds {
                column: 2,
                width: 2
            }
        );
    }

    #[test]
    fn filter_null_keeps_matching_weighted_rows() {
        let is_null = DataflowAutomaton::new(vec![DataflowOperator::FilterNull {
            column: 1,
            predicate: NullPredicate::IsNull,
        }]);
        let is_not_null = DataflowAutomaton::new(vec![DataflowOperator::FilterNull {
            column: 1,
            predicate: NullPredicate::IsNotNull,
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(1), SqliteValue::Null], 3),
            WeightedRow::new(vec![int(2), int(20)], -2),
            WeightedRow::new(vec![int(3), SqliteValue::Null], 0),
        ];

        assert_eq!(
            is_null.execute(&rows).expect("is-null stream"),
            vec![WeightedRow::new(vec![int(1), SqliteValue::Null], 3)]
        );
        assert_eq!(
            is_not_null.execute(&rows).expect("is-not-null stream"),
            vec![WeightedRow::new(vec![int(2), int(20)], -2)]
        );
    }

    #[test]
    fn filter_null_rejects_out_of_bounds_columns() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::FilterNull {
            column: 2,
            predicate: NullPredicate::IsNull,
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), int(2)])])
            .expect_err("invalid null predicate column should fail");

        assert_eq!(
            err,
            DataflowError::ColumnOutOfBounds {
                column: 2,
                width: 2
            }
        );
    }

    #[test]
    fn filter_weight_sign_keeps_requested_delta_side_and_elides_zeroes() {
        let positive = DataflowAutomaton::new(vec![DataflowOperator::FilterWeightSign {
            sign: WeightSign::Positive,
        }]);
        let negative = DataflowAutomaton::new(vec![DataflowOperator::FilterWeightSign {
            sign: WeightSign::Negative,
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(1)], 3),
            WeightedRow::new(vec![int(2)], -2),
            WeightedRow::new(vec![int(3)], 0),
        ];

        assert_eq!(
            positive.execute(&rows).expect("positive stream"),
            vec![WeightedRow::new(vec![int(1)], 3)]
        );
        assert_eq!(
            negative.execute(&rows).expect("negative stream"),
            vec![WeightedRow::new(vec![int(2)], -2)]
        );
    }

    #[test]
    fn append_literal_column_preserves_weights_and_elides_zeroes() {
        let automaton =
            DataflowAutomaton::new(vec![DataflowOperator::AppendLiteral { value: int(99) }]);
        let rows = vec![
            WeightedRow::new(vec![int(1)], 3),
            WeightedRow::new(vec![int(2)], -2),
            WeightedRow::new(vec![int(3)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::new(vec![int(1), int(99)], 3),
                WeightedRow::new(vec![int(2), int(99)], -2),
            ]
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
    fn count_by_key_appends_counts_and_elides_zero_groups() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::CountByKey {
            key_columns: vec![0],
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(2), int(20)], 1),
            WeightedRow::new(vec![int(1), int(10)], 4),
            WeightedRow::new(vec![int(2), int(21)], -1),
            WeightedRow::new(vec![int(1), int(11)], 3),
            WeightedRow::new(vec![int(3), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(actual, vec![WeightedRow::insert(vec![int(1), int(7)])]);
    }

    #[test]
    fn count_by_key_rejects_out_of_bounds_key_columns() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::CountByKey {
            key_columns: vec![2],
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), int(2)])])
            .expect_err("invalid key column should fail");

        assert_eq!(
            err,
            DataflowError::ColumnOutOfBounds {
                column: 2,
                width: 2
            }
        );
    }

    #[test]
    fn sum_integer_by_key_appends_weighted_sums_and_elides_zero_groups() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::SumIntegerByKey {
            key_columns: vec![0],
            value_column: 1,
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(2), int(20)], 1),
            WeightedRow::new(vec![int(1), int(10)], 4),
            WeightedRow::new(vec![int(2), int(20)], -1),
            WeightedRow::new(vec![int(1), int(11)], 3),
            WeightedRow::new(vec![int(3), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(actual, vec![WeightedRow::insert(vec![int(1), int(73)])]);
    }

    #[test]
    fn sum_integer_by_key_rejects_non_integer_values() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::SumIntegerByKey {
            key_columns: vec![0],
            value_column: 1,
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), SqliteValue::Float(1.5)])])
            .expect_err("non-integer aggregate input should fail");

        assert_eq!(err, DataflowError::AggregateValueNotInteger { column: 1 });
    }

    #[test]
    fn min_integer_by_key_appends_minimum_for_positive_rows() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::MinIntegerByKey {
            key_columns: vec![0],
            value_column: 1,
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(2), int(20)], 1),
            WeightedRow::new(vec![int(1), int(10)], 1),
            WeightedRow::new(vec![int(2), int(7)], 1),
            WeightedRow::new(vec![int(1), int(3)], -1),
            WeightedRow::new(vec![int(3), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::insert(vec![int(2), int(7)]),
                WeightedRow::insert(vec![int(1), int(10)]),
            ]
        );
    }

    #[test]
    fn min_integer_by_key_rejects_non_integer_values() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::MinIntegerByKey {
            key_columns: vec![0],
            value_column: 1,
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), SqliteValue::Float(1.5)])])
            .expect_err("non-integer aggregate input should fail");

        assert_eq!(err, DataflowError::AggregateValueNotInteger { column: 1 });
    }

    #[test]
    fn max_integer_by_key_appends_maximum_for_positive_rows() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::MaxIntegerByKey {
            key_columns: vec![0],
            value_column: 1,
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(2), int(20)], 1),
            WeightedRow::new(vec![int(1), int(10)], 1),
            WeightedRow::new(vec![int(2), int(7)], 1),
            WeightedRow::new(vec![int(1), int(30)], -1),
            WeightedRow::new(vec![int(3), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::insert(vec![int(2), int(20)]),
                WeightedRow::insert(vec![int(1), int(10)]),
            ]
        );
    }

    #[test]
    fn max_integer_by_key_rejects_non_integer_values() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::MaxIntegerByKey {
            key_columns: vec![0],
            value_column: 1,
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), SqliteValue::Float(1.5)])])
            .expect_err("non-integer aggregate input should fail");

        assert_eq!(err, DataflowError::AggregateValueNotInteger { column: 1 });
    }

    #[test]
    fn average_integer_by_key_appends_average_for_positive_rows() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::AverageIntegerByKey {
            key_columns: vec![0],
            value_column: 1,
        }]);
        let rows = vec![
            WeightedRow::new(vec![int(2), int(20)], 2),
            WeightedRow::new(vec![int(1), int(10)], 1),
            WeightedRow::new(vec![int(2), int(8)], 1),
            WeightedRow::new(vec![int(1), int(30)], -1),
            WeightedRow::new(vec![int(3), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::insert(vec![int(2), SqliteValue::Float(16.0)]),
                WeightedRow::insert(vec![int(1), SqliteValue::Float(10.0)]),
            ]
        );
    }

    #[test]
    fn average_integer_by_key_rejects_non_integer_values() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::AverageIntegerByKey {
            key_columns: vec![0],
            value_column: 1,
        }]);

        let err = automaton
            .execute(&[WeightedRow::insert(vec![int(1), SqliteValue::Float(1.5)])])
            .expect_err("non-integer aggregate input should fail");

        assert_eq!(err, DataflowError::AggregateValueNotInteger { column: 1 });
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
    fn set_weight_operator_normalizes_surviving_row_weights() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::SetWeight { weight: 1 }]);
        let rows = vec![
            WeightedRow::new(vec![int(1), int(10)], 3),
            WeightedRow::new(vec![int(2), int(20)], -2),
            WeightedRow::new(vec![int(3), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::insert(vec![int(1), int(10)]),
                WeightedRow::insert(vec![int(2), int(20)]),
            ]
        );
    }

    #[test]
    fn set_weights_zero_output_weight_elides_all_rows() {
        let rows = vec![
            WeightedRow::new(vec![int(1)], 3),
            WeightedRow::new(vec![int(2)], -2),
        ];

        let actual = set_weights(&rows, 0);

        assert_eq!(actual, Vec::<WeightedRow>::new());
    }

    #[test]
    fn threshold_positive_consolidates_and_normalizes_membership() {
        let rows = vec![
            WeightedRow::new(vec![int(1), int(10)], 2),
            WeightedRow::new(vec![int(2), int(20)], 1),
            WeightedRow::new(vec![int(1), int(10)], -1),
            WeightedRow::new(vec![int(2), int(20)], -1),
            WeightedRow::new(vec![int(3), int(30)], -4),
            WeightedRow::new(vec![int(4), int(40)], 0),
        ];

        let actual = threshold_positive(&rows);

        assert_eq!(actual, vec![WeightedRow::insert(vec![int(1), int(10)])]);
    }

    #[test]
    fn threshold_positive_operator_can_follow_weight_scaling() {
        let automaton = DataflowAutomaton::new(vec![
            DataflowOperator::ScaleWeight { factor: -1 },
            DataflowOperator::ThresholdPositive,
        ]);
        let rows = vec![
            WeightedRow::new(vec![int(1)], -3),
            WeightedRow::new(vec![int(2)], 2),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(actual, vec![WeightedRow::insert(vec![int(1)])]);
    }

    #[test]
    fn append_weight_column_materializes_delta_weights_for_emission() {
        let automaton = DataflowAutomaton::new(vec![DataflowOperator::AppendWeightColumn]);
        let rows = vec![
            WeightedRow::new(vec![int(1), int(10)], 3),
            WeightedRow::new(vec![int(2), int(20)], -2),
            WeightedRow::new(vec![int(3), int(30)], 0),
        ];

        let actual = automaton.execute(&rows).expect("dataflow should execute");

        assert_eq!(
            actual,
            vec![
                WeightedRow::insert(vec![int(1), int(10), int(3)]),
                WeightedRow::insert(vec![int(2), int(20), int(-2)]),
            ]
        );
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
