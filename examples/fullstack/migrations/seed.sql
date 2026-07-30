-- Seed: тестовые данные для fullstack-примера.
-- Применяется: dm db seed  (или dm db reset)

INSERT INTO users (email, name) VALUES
    ('alice@example.com', 'Alice'),
    ('bob@example.com', 'Bob')
ON CONFLICT (email) DO NOTHING;
