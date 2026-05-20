use fsqlite_ast::{SelectStatement, Statement};
use fsqlite_parser::parse_first_statement_with_tail;
use fsqlite_vdbe::{
    ProgramBuilder,
    codegen::{CodegenContext, ColumnInfo, IndexSchema, TableSchema, codegen_select},
};

fn parse_select(sql: &str) -> Result<SelectStatement, String> {
    let Some((stmt, consumed)) =
        parse_first_statement_with_tail(sql).map_err(|error| error.to_string())?
    else {
        return Err("expected a parsed SELECT statement".to_owned());
    };
    if consumed != sql.len() {
        return Err(format!(
            "parser consumed {consumed} bytes from a {} byte statement",
            sql.len()
        ));
    }

    match stmt {
        Statement::Select(select) => Ok(select),
        other => Err(format!("expected SELECT statement, got {other:?}")),
    }
}

fn column(name: &str, affinity: char, is_ipk: bool) -> ColumnInfo {
    ColumnInfo::basic(name, affinity, is_ipk)
}

fn index(name: &str, root_page: i32, columns: &[&str], is_unique: bool) -> IndexSchema {
    IndexSchema {
        name: name.to_owned(),
        root_page,
        columns: columns.iter().map(|column| (*column).to_owned()).collect(),
        key_expressions: Vec::new(),
        key_sort_directions: Vec::new(),
        where_clause: None,
        is_unique,
        key_collations: Vec::new(),
    }
}

fn table(
    name: &str,
    root_page: i32,
    columns: Vec<ColumnInfo>,
    indexes: Vec<IndexSchema>,
) -> TableSchema {
    TableSchema {
        name: name.to_owned(),
        root_page,
        columns,
        indexes,
        strict: false,
        without_rowid: false,
        primary_key_constraints: Vec::new(),
        foreign_keys: Vec::new(),
        check_constraints: Vec::new(),
    }
}

fn snapshot_schema() -> Vec<TableSchema> {
    vec![
        table(
            "docs",
            2,
            vec![
                column("id", 'D', true),
                column("category_id", 'D', false),
                column("title", 'B', false),
                column("body", 'B', false),
                column("score", 'D', false),
            ],
            vec![
                index("idx_docs_category", 10, &["category_id"], false),
                index("idx_docs_title", 11, &["title"], false),
            ],
        ),
        table(
            "categories",
            3,
            vec![column("id", 'D', true), column("name", 'B', false)],
            vec![index("idx_categories_name", 12, &["name"], false)],
        ),
        table(
            "events",
            4,
            vec![
                column("id", 'D', true),
                column("category_id", 'D', false),
                column("score", 'D', false),
            ],
            vec![index("idx_events_category", 13, &["category_id"], false)],
        ),
    ]
}

fn render_bytecode_case(name: &str, sql: &str) -> Result<String, String> {
    let select = parse_select(sql)?;
    let schema = snapshot_schema();
    let mut builder = ProgramBuilder::new();
    let mut output = String::new();

    output.push_str(&format!("case: {name}\n"));
    output.push_str(&format!("sql: {sql}\n"));

    match codegen_select(&mut builder, &select, &schema, &CodegenContext::default()) {
        Ok(()) => match builder.finish() {
            Ok(program) => {
                output.push_str("status: ok\n");
                output.push_str(&format!("registers: {}\n", program.register_count()));
                match program.max_bind_parameter_index() {
                    Ok(max_index) => {
                        output.push_str(&format!("bind_parameters: {max_index}\n"));
                    }
                    Err(bad_index) => {
                        output.push_str(&format!("bind_parameters_error: {bad_index}\n"));
                    }
                }
                output.push_str(&format!(
                    "requires_attached_memdb: {}\n",
                    program.requires_attached_memdb()
                ));
                output.push_str(&format!(
                    "requires_version_store: {}\n",
                    program.requires_version_store()
                ));
                output.push('\n');
                output.push_str(&program.disassemble());
            }
            Err(error) => {
                output.push_str("status: builder_error\n");
                output.push_str(&format!("error: {error}\n"));
            }
        },
        Err(error) => {
            output.push_str("status: codegen_error\n");
            output.push_str(&format!("error: {error}\n"));
        }
    }

    Ok(output)
}

#[test]
fn golden_vdbe_select_bytecode_recent_stub_shapes() -> Result<(), String> {
    let join_lookup = render_bytecode_case(
        "join_lookup",
        "SELECT docs.id, categories.name \
             FROM docs JOIN categories ON docs.category_id = categories.id \
             WHERE docs.category_id = 7",
    )?;
    insta::assert_snapshot!("select_join_lookup", join_lookup);

    let grouped_aggregate = render_bytecode_case(
        "grouped_aggregate",
        "SELECT category_id, count(*), sum(score) \
             FROM docs GROUP BY category_id HAVING sum(score) > 10",
    )?;
    insta::assert_snapshot!("select_grouped_aggregate", grouped_aggregate);

    let match_predicate = render_bytecode_case(
        "match_predicate",
        "SELECT id, title FROM docs WHERE title MATCH ?1",
    )?;
    insta::assert_snapshot!("select_match_predicate", match_predicate);

    let window_boundary = render_bytecode_case(
        "window_boundary",
        "SELECT category_id, \
                    sum(score) OVER (PARTITION BY category_id ORDER BY score) \
             FROM events",
    )?;
    insta::assert_snapshot!("select_window_boundary", window_boundary);

    Ok(())
}

#[test]
fn golden_vdbe_fts5_match_bytecode_family() -> Result<(), String> {
    let body_match_with_filter = render_bytecode_case(
        "fts5_body_match_with_filter",
        "SELECT id, title, score \
             FROM docs \
             WHERE body MATCH ?1 AND category_id = 3 \
             ORDER BY score DESC",
    )?;
    insta::assert_snapshot!(
        "fts5_body_match_with_filter_ordered",
        body_match_with_filter
    );

    let title_match_with_limit = render_bytecode_case(
        "fts5_title_match_with_limit",
        "SELECT id, title \
             FROM docs \
             WHERE title MATCH 'franken sqlite' \
             ORDER BY id \
             LIMIT 5",
    )?;
    insta::assert_snapshot!("fts5_title_match_with_limit", title_match_with_limit);

    Ok(())
}
