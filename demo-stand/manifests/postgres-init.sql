-- Demo data for KubeTile Postgres pod
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS users;

CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  email TEXT NOT NULL UNIQUE,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  joined_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE orders (
  id SERIAL PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id),
  total NUMERIC(8,2) NOT NULL,
  placed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  fulfilled BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE order_items (
  id SERIAL PRIMARY KEY,
  order_id INTEGER NOT NULL REFERENCES orders(id),
  sku TEXT NOT NULL,
  quantity INTEGER NOT NULL,
  metadata JSONB
);

INSERT INTO users (name, email, is_active, joined_at) VALUES
  ('Avery Lowry', 'avery.lowry@example.com', TRUE, NOW() - INTERVAL '21 days'),
  ('Basil Chow', 'basil.chow@example.com', TRUE, NOW() - INTERVAL '14 days'),
  ('Carmen Prado', 'carmen.prado@example.com', FALSE, NOW() - INTERVAL '90 days'),
  ('Devon Muir', 'devon.muir@example.com', TRUE, NOW() - INTERVAL '7 days'),
  ('Elise Bran', 'elise.bran@example.com', TRUE, NOW() - INTERVAL '3 days');

INSERT INTO orders (user_id, total, placed_at, fulfilled) VALUES
  (1, 128.50, NOW() - INTERVAL '20 days', TRUE),
  (1, 22.99, NOW() - INTERVAL '19 days', TRUE),
  (2, 44.25, NOW() - INTERVAL '13 days', FALSE),
  (3, 150.00, NOW() - INTERVAL '40 days', TRUE),
  (4, 18.35, NOW() - INTERVAL '6 days', FALSE),
  (5, 79.99, NOW() - INTERVAL '2 days', FALSE),
  (2, 4.50, NOW() - INTERVAL '1 day', FALSE);

INSERT INTO order_items (order_id, sku, quantity, metadata) VALUES
  (1, 'NB-ALPHA-01', 1, '{"color": "obsidian", "pack": "starter"}'),
  (1, 'NB-ALPHA-02', 2, '{"color": "ember", "pack": "starter"}'),
  (2, 'MG-RED-99', 1, '{"color": "ruby", "tags": ["gift"]}'),
  (3, 'FW-CORE-77', 3, '{"warranty_months": 24}'),
  (4, 'FW-CORE-77', 1, '{"warranty_months": 24}'),
  (5, 'TB-SET-11', 2, '{"bundle": true, "items": ["mouse", "pad"]}'),
  (6, 'SP-CLIP-02', 5, '{"color": "silver"}'),
  (7, 'MG-RED-99', 1, '{"color": "ruby", "tags": ["promo"]}'),
  (7, 'SP-CLIP-02', 2, '{"color": "silver", "bundle": "mix"}');
