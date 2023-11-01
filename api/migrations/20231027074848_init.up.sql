create table if not exists price (
    id uuid primary key,

    base int not null,

    egg int not null,
    cheese int not null,
    spicy_mayonnaise int not null,

    no_mayonnaise int not null,
    no_sauce int not null,
    no_bonito int not null, -- かつおぶし
    no_aonori int not null,

    created_at timestamptz not null default current_timestamp
);

insert into price (id, base, egg, cheese, spicy_mayonnaise, no_mayonnaise, no_sauce, no_bonito, no_aonori)
    values ('018b7b2a-a5de-7ed0-98fa-11ffb9e43e67', 300, 50, 50, 50, 0, 0, 0, 0);

create table if not exists order_group (
    id uuid primary key,

    price_table_id uuid not null references price(id),

    created_at timestamptz not null default current_timestamp
);

create table if not exists orders (
    id uuid primary key,
    group_id uuid not null references order_group(id),

    egg boolean not null,
    cheese boolean not null,
    spicy_mayonnaise boolean not null,

    no_mayonnaise boolean not null,
    no_sauce boolean not null,
    no_bonito boolean not null, -- かつおぶし
    no_aonori boolean not null,

    created_at timestamptz not null default current_timestamp
);

create index orders__group_id on orders(group_id);

create table if not exists order_payed (
    order_group_id uuid primary key references order_group(id),
    payed_amount int not null, -- 受け取った金額
    created_at timestamptz not null default current_timestamp
);

create table if not exists order_queued (
    order_id uuid primary key references orders(id),
    wait_number serial not null,
    created_at timestamptz not null default current_timestamp
);

create table if not exists order_assigned (
    order_id uuid primary key references orders(id),
    chef_number int not null,
    created_at timestamptz not null default current_timestamp
);

create table if not exists order_ready (
    order_id uuid primary key references orders(id),
    created_at timestamptz not null default current_timestamp
);

create table if not exists order_delivered (
    order_id uuid primary key references orders(id),
    created_at timestamptz not null default current_timestamp
);
