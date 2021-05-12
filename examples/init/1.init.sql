SELECT '{{ schema }}';
-->
--rmig--{"run_always": true}
CREATE TABLE IF NOT EXISTS {{ SCHEMA_ADMIN }}.rmigtest(ID numeric);
-->
INSERT INTO {{ SCHEMA_ADMIN }}.rmigtest(ID) VALUES (1);
-->
INSERT INTO {{ SCHEMA_ADMIN }}.rmigtest(ID) VALUES (2);
-->
INSERT INTO {{ SCHEMA_ADMIN }}.rmigtest(ID) VALUES (3);
-->
INSERT INTO {{ SCHEMA_ADMIN }}.rmigtest(ID) VALUES (4);
-->
--rmig--{"run_always": true}
drop table {{ SCHEMA_ADMIN }}.rmigtest;