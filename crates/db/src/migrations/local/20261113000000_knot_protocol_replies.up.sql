alter table knot_message_receipts
    add column request jsonb,
    add column reply jsonb;
