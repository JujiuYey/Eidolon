import type { SchemaDbType } from '@/types/schema-dict';

/**
 * 智能查询模板：用户在自己的 SQL 客户端里执行，把结果粘贴回来。
 * 输出列名对齐后端 parse_query_result 识别的字段别名。
 */
export const SCHEMA_QUERY_TEMPLATES: Record<SchemaDbType, string> = {
  pg: `SELECT
    c.table_name,
    c.ordinal_position,
    c.column_name,
    c.data_type,
    c.is_nullable,
    c.column_default,
    COALESCE(col_description(format('"%s"."%s"', c.table_schema, c.table_name)::regclass, c.ordinal_position), '') AS column_comment,
    COALESCE((
        SELECT 'Y'
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
          ON tc.constraint_name = kcu.constraint_name
         AND tc.table_schema = kcu.table_schema
        WHERE tc.constraint_type = 'PRIMARY KEY'
          AND tc.table_schema = c.table_schema
          AND tc.table_name = c.table_name
          AND kcu.column_name = c.column_name
        LIMIT 1
    ), 'N') AS primary_key
FROM information_schema.columns c
WHERE c.table_schema = 'public'
ORDER BY c.table_name, c.ordinal_position;`,

  mysql: `SELECT
    c.TABLE_NAME AS table_name,
    c.ORDINAL_POSITION AS ordinal_position,
    c.COLUMN_NAME AS column_name,
    c.COLUMN_TYPE AS data_type,
    c.IS_NULLABLE AS is_nullable,
    c.COLUMN_DEFAULT AS column_default,
    c.COLUMN_COMMENT AS column_comment,
    c.COLUMN_KEY AS primary_key
FROM information_schema.COLUMNS c
WHERE c.TABLE_SCHEMA = DATABASE()
ORDER BY c.TABLE_NAME, c.ORDINAL_POSITION;`,

  oracle: `SELECT
    cols.table_name,
    cols.column_id AS ordinal_position,
    cols.column_name,
    cols.data_type || CASE
        WHEN cols.data_precision IS NOT NULL AND cols.data_scale > 0 THEN '(' || cols.data_precision || ',' || cols.data_scale || ')'
        WHEN cols.data_precision IS NOT NULL THEN '(' || cols.data_precision || ')'
        WHEN cols.char_length > 0 THEN '(' || cols.char_length || ')'
        ELSE ''
    END AS data_type,
    cols.nullable AS is_nullable,
    cols.data_default AS column_default,
    COALESCE(comments.comments, '') AS column_comment,
    CASE WHEN pk.column_name IS NOT NULL THEN 'Y' ELSE 'N' END AS primary_key
FROM user_tab_columns cols
LEFT JOIN user_col_comments comments
       ON comments.table_name = cols.table_name
      AND comments.column_name = cols.column_name
LEFT JOIN (
    SELECT cc.table_name, cc.column_name
    FROM user_constraints cons
    JOIN user_cons_columns cc ON cc.constraint_name = cons.constraint_name
    WHERE cons.constraint_type = 'P'
) pk ON pk.table_name = cols.table_name AND pk.column_name = cols.column_name
ORDER BY cols.table_name, cols.column_id;`,
};

export const SCHEMA_DB_TYPE_LABELS: Record<SchemaDbType, string> = {
  mysql: 'MySQL',
  pg: 'PostgreSQL',
  oracle: 'Oracle',
};
