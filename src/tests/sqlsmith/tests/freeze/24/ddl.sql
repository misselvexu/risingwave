CREATE TABLE supplier (s_suppkey INT, s_name CHARACTER VARYING, s_address CHARACTER VARYING, s_nationkey INT, s_phone CHARACTER VARYING, s_acctbal NUMERIC, s_comment CHARACTER VARYING, PRIMARY KEY (s_suppkey));
CREATE TABLE part (p_partkey INT, p_name CHARACTER VARYING, p_mfgr CHARACTER VARYING, p_brand CHARACTER VARYING, p_type CHARACTER VARYING, p_size INT, p_container CHARACTER VARYING, p_retailprice NUMERIC, p_comment CHARACTER VARYING, PRIMARY KEY (p_partkey));
CREATE TABLE partsupp (ps_partkey INT, ps_suppkey INT, ps_availqty INT, ps_supplycost NUMERIC, ps_comment CHARACTER VARYING, PRIMARY KEY (ps_partkey, ps_suppkey));
CREATE TABLE customer (c_custkey INT, c_name CHARACTER VARYING, c_address CHARACTER VARYING, c_nationkey INT, c_phone CHARACTER VARYING, c_acctbal NUMERIC, c_mktsegment CHARACTER VARYING, c_comment CHARACTER VARYING, PRIMARY KEY (c_custkey));
CREATE TABLE orders (o_orderkey BIGINT, o_custkey INT, o_orderstatus CHARACTER VARYING, o_totalprice NUMERIC, o_orderdate DATE, o_orderpriority CHARACTER VARYING, o_clerk CHARACTER VARYING, o_shippriority INT, o_comment CHARACTER VARYING, PRIMARY KEY (o_orderkey));
CREATE TABLE lineitem (l_orderkey BIGINT, l_partkey INT, l_suppkey INT, l_linenumber INT, l_quantity NUMERIC, l_extendedprice NUMERIC, l_discount NUMERIC, l_tax NUMERIC, l_returnflag CHARACTER VARYING, l_linestatus CHARACTER VARYING, l_shipdate DATE, l_commitdate DATE, l_receiptdate DATE, l_shipinstruct CHARACTER VARYING, l_shipmode CHARACTER VARYING, l_comment CHARACTER VARYING, PRIMARY KEY (l_orderkey, l_linenumber));
CREATE TABLE nation (n_nationkey INT, n_name CHARACTER VARYING, n_regionkey INT, n_comment CHARACTER VARYING, PRIMARY KEY (n_nationkey));
CREATE TABLE region (r_regionkey INT, r_name CHARACTER VARYING, r_comment CHARACTER VARYING, PRIMARY KEY (r_regionkey));
CREATE TABLE person (id BIGINT, name CHARACTER VARYING, email_address CHARACTER VARYING, credit_card CHARACTER VARYING, city CHARACTER VARYING, state CHARACTER VARYING, date_time TIMESTAMP, extra CHARACTER VARYING, PRIMARY KEY (id));
CREATE TABLE auction (id BIGINT, item_name CHARACTER VARYING, description CHARACTER VARYING, initial_bid BIGINT, reserve BIGINT, date_time TIMESTAMP, expires TIMESTAMP, seller BIGINT, category BIGINT, extra CHARACTER VARYING, PRIMARY KEY (id));
CREATE TABLE bid (auction BIGINT, bidder BIGINT, price BIGINT, channel CHARACTER VARYING, url CHARACTER VARYING, date_time TIMESTAMP, extra CHARACTER VARYING);
CREATE TABLE alltypes1 (c1 BOOLEAN, c2 SMALLINT, c3 INT, c4 BIGINT, c5 REAL, c6 DOUBLE, c7 NUMERIC, c8 DATE, c9 CHARACTER VARYING, c10 TIME, c11 TIMESTAMP, c13 INTERVAL, c14 STRUCT<a INT>, c15 INT[], c16 CHARACTER VARYING[]);
CREATE TABLE alltypes2 (c1 BOOLEAN, c2 SMALLINT, c3 INT, c4 BIGINT, c5 REAL, c6 DOUBLE, c7 NUMERIC, c8 DATE, c9 CHARACTER VARYING, c10 TIME, c11 TIMESTAMP, c13 INTERVAL, c14 STRUCT<a INT>, c15 INT[], c16 CHARACTER VARYING[]);
CREATE MATERIALIZED VIEW m0 AS SELECT '06jQ5OqeK7' AS col_0, ((coalesce(NULL, NULL, NULL, (SMALLINT '605'), NULL, NULL, NULL, NULL, NULL, NULL)) - (INT '771')) AS col_1 FROM partsupp AS t_0 WHERE true GROUP BY t_0.ps_partkey, t_0.ps_comment, t_0.ps_availqty;
CREATE MATERIALIZED VIEW m2 AS WITH with_0 AS (SELECT t_1.c8 AS col_0, (ARRAY['f5dpz5Kt5j']) AS col_1, CAST(NULL AS STRUCT<a INT>) AS col_2 FROM alltypes2 AS t_1 WHERE false GROUP BY t_1.c16, t_1.c8, t_1.c11, t_1.c14, t_1.c7, t_1.c2, t_1.c10, t_1.c13) SELECT (SMALLINT '0') AS col_0, false AS col_1, ARRAY[(BIGINT '232'), (BIGINT '839'), (BIGINT '692'), (BIGINT '251')] AS col_2, (-2147483648) AS col_3 FROM with_0;
CREATE MATERIALIZED VIEW m3 AS SELECT hop_0.c15 AS col_0 FROM hop(alltypes1, alltypes1.c11, INTERVAL '3600', INTERVAL '43200') AS hop_0 GROUP BY hop_0.c15, hop_0.c8, hop_0.c10, hop_0.c6, hop_0.c14, hop_0.c9 HAVING true;
CREATE MATERIALIZED VIEW m4 AS SELECT t_1.ps_partkey AS col_0, t_0.c3 AS col_1 FROM alltypes1 AS t_0 LEFT JOIN partsupp AS t_1 ON t_0.c3 = t_1.ps_suppkey AND t_0.c1 GROUP BY t_0.c3, t_1.ps_partkey, t_0.c5;
CREATE MATERIALIZED VIEW m5 AS WITH with_0 AS (SELECT t_1.ps_comment AS col_0 FROM partsupp AS t_1 GROUP BY t_1.ps_comment) SELECT (SMALLINT '259') AS col_0 FROM with_0 WHERE true;
CREATE MATERIALIZED VIEW m6 AS SELECT (FLOAT '611') AS col_0, (455) AS col_1, tumble_0.c1 AS col_2 FROM tumble(alltypes1, alltypes1.c11, INTERVAL '24') AS tumble_0 WHERE tumble_0.c1 GROUP BY tumble_0.c1, tumble_0.c13, tumble_0.c16, tumble_0.c14, tumble_0.c7;
CREATE MATERIALIZED VIEW m7 AS SELECT DATE '2022-05-13' AS col_0, tumble_0.email_address AS col_1 FROM tumble(person, person.date_time, INTERVAL '82') AS tumble_0 WHERE ((INT '299') = (INT '2147483647')) GROUP BY tumble_0.city, tumble_0.name, tumble_0.email_address, tumble_0.state HAVING max(((REAL '5') < (INT '200')));
CREATE MATERIALIZED VIEW m8 AS SELECT sq_2.col_0 AS col_0, (sq_2.col_0 + ((SMALLINT '311') # (SMALLINT '522'))) AS col_1, sq_2.col_0 AS col_2 FROM (SELECT ((INT '713')) AS col_0 FROM m0 AS t_0 LEFT JOIN partsupp AS t_1 ON t_0.col_1 = t_1.ps_availqty WHERE true GROUP BY t_1.ps_suppkey HAVING false) AS sq_2 GROUP BY sq_2.col_0;
CREATE MATERIALIZED VIEW m9 AS SELECT (TRIM(sq_1.col_3)) AS col_0, sq_1.col_3 AS col_1, sq_1.col_3 AS col_2 FROM (SELECT min(true) AS col_0, ((BIGINT '681') | (BIGINT '697')) AS col_1, (SMALLINT '0') AS col_2, (lower(t_0.col_0)) AS col_3 FROM m0 AS t_0 WHERE false GROUP BY t_0.col_0) AS sq_1 WHERE sq_1.col_0 GROUP BY sq_1.col_3 HAVING false;