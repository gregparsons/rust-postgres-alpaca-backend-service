-- noinspection SqlNoDataSourceInspectionForFile

-- noinspection SqlDialectInspectionForFile

-- alpaca_position table

create table if not exists alpaca_position
(
    symbol varchar not null
        constraint alpaca_position_pk
            primary key,
    exchange varchar,
    asset_class varchar,
    avg_entry_price numeric(20,10),
    qty numeric(20,10),
    qty_available numeric(20,10),
    side varchar,
    market_value numeric(20,10),
    cost_basis numeric(20,10),
    unrealized_pl numeric(20,10),
    unrealized_plpc numeric(20,10),
    current_price numeric(20,10),
    lastday_price numeric(20,10),
    change_today numeric(20,10),
    asset_id varchar
);

alter table alpaca_position owner to postgres;
