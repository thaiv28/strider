-- One-time crossover of the original owner's pantry to USDA reference foods.
-- Reviewed mapping; rewrites kcal and density. No-op on any database without that workspace.
UPDATE ingredient SET kcal_per_100g = 313, density_g_ml = 0.122, density_source = 'usda', source = 'usda', source_id = '170917', reference_name = 'Spices, bay leaf' WHERE id = 37 AND user_id = 1 AND name = 'Bay leaf';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 380, density_g_ml = 0.93, density_source = 'usda', source = 'usda', source_id = '168833', reference_name = 'Sugars, brown' WHERE id = 27 AND user_id = 1 AND name = 'BROWN SUGAR';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 371, density_g_ml = 0.271, density_source = 'usda', source = 'usda', source_id = '169736', reference_name = 'Pasta, dry, enriched' WHERE id = 48 AND user_id = 1 AND name = 'Cavatappi';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 267, density_g_ml = 0.406, density_source = 'usda', source = 'usda', source_id = '171562', reference_name = 'Soup, chicken broth or bouillon, dry' WHERE id = 34 AND user_id = 1 AND name = 'Chicken Bouillon';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 185, density_g_ml = 0.866, density_source = 'usda', source = 'usda', source_id = '171110', reference_name = 'Chicken, canned, no broth' WHERE id = 43 AND user_id = 1 AND name = 'Chicken, Canned Chicken Breast';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 536, density_g_ml = 0.19, density_source = 'usda', source = 'usda', source_id = '2708199', reference_name = 'Corn chips, flavored (Fritos)' WHERE id = 30 AND user_id = 1 AND name = 'Corn chips, flavored (Fritos)';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 536, density_g_ml = 0.19, density_source = 'usda', source = 'usda', source_id = '2708199', reference_name = 'Corn chips, flavored (Fritos)' WHERE id = 29 AND user_id = 1 AND name = '4.62OZ FRITOS HONEY BBQ';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 349, density_g_ml = 0.237, density_source = 'usda', source = 'usda', source_id = '170002', reference_name = 'Onions, dehydrated flakes' WHERE id = 36 AND user_id = 1 AND name = 'Minced Onion';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 341, density_g_ml = 0.467, density_source = 'usda', source = 'usda', source_id = '171327', reference_name = 'Spices, onion powder' WHERE id = 47 AND user_id = 1 AND name = 'Granulated onion';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 331, density_g_ml = 0.656, density_source = 'usda', source = 'usda', source_id = '171325', reference_name = 'Spices, garlic powder' WHERE id = 35 AND user_id = 1 AND name = 'Freeze Dried Garlic';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 496, density_g_ml = 0.541, density_source = 'usda', source = 'usda', source_id = '170876', reference_name = 'Milk, dry, whole, with added vitamin D' WHERE id = 45 AND user_id = 1 AND name = 'Dry milk';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 387, density_g_ml = 0.845, density_source = 'usda', source = 'usda', source_id = '169655', reference_name = 'Sugars, granulated' WHERE id = 39 AND user_id = 1 AND name = 'Granulated Sugar';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 251, density_g_ml = 0.467, density_source = 'usda', source = 'usda', source_id = '170931', reference_name = 'Spices, pepper, black' WHERE id = 38 AND user_id = 1 AND name = 'Ground black pepper';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 380, density_g_ml = 0.402, density_source = 'usda', source = 'usda', source_id = '169709', reference_name = 'Rice, white, long-grain, precooked or instant, enriched, dry' WHERE id = 33 AND user_id = 1 AND name = 'Instant rice';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 420, density_g_ml = 0.423, density_source = 'usda', source = 'usda', source_id = '171247', reference_name = 'Cheese, parmesan, grated' WHERE id = 46 AND user_id = 1 AND name = 'Kraft Parmesan Cheese';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 884, density_g_ml = 0.913, density_source = 'usda', source = 'usda', source_id = '171413', reference_name = 'Oil, olive, salad or cooking' WHERE id = 42 AND user_id = 1 AND name = 'Organic Extra Virgin Olive Oil';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 598, density_g_ml = 1.091, density_source = 'usda', source = 'usda', source_id = '174266', reference_name = 'Peanut butter, smooth style, with salt' WHERE id = 28 AND user_id = 1 AND name = 'Peanut butter';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 18, density_g_ml = 1.006, density_source = 'usda', source = 'usda', source_id = '172237', reference_name = 'Vinegar, distilled' WHERE id = 41 AND user_id = 1 AND name = 'Vinegar';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 53, density_g_ml = 1.078, density_source = 'usda', source = 'usda', source_id = '174277', reference_name = 'Soy sauce made from soy and wheat (shoyu)' WHERE id = 40 AND user_id = 1 AND name = 'Soy Sauce';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 319, density_g_ml = 0.676, density_source = 'usda', source = 'usda', source_id = '2709205', reference_name = 'Mango, dried' WHERE id = 31 AND user_id = 1 AND name = 'DRIED MANGO';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 450, density_g_ml = NULL, density_source = NULL, source = 'usda', source_id = '172725', reference_name = 'Cookies, oatmeal, commercially prepared, regular' WHERE id = 32 AND user_id = 1 AND name = 'Oatmea cookie chuncks';
--> statement-breakpoint
UPDATE ingredient SET kcal_per_100g = 462, density_g_ml = 0.634, density_source = 'usda', source = 'usda', source_id = '167561', reference_name = 'Snacks, trail mix, regular' WHERE id = 44 AND user_id = 1 AND name = 'Trail mix';
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Chex Mix', 417, 0.254, 'usda', 'usda', '2708291', 'Snack mix, plain (Chex Mix)' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Chex Mix');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Coconut Strips', 456, 0.359, 'usda', 'usda', '170577', 'Nuts, coconut meat, dried (desiccated), sweetened, flaked, packaged' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Coconut Strips');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Crushed Nuts', 594, 0.579, 'usda', 'usda', '168599', 'Nuts, mixed nuts, dry roasted, with peanuts, with salt added' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Crushed Nuts');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Dried Cranberries', 308, 0.676, 'usda', 'usda', '171723', 'Cranberries, dried, sweetened' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Dried Cranberries');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Golden Raisins', 301, 0.613, 'usda', 'usda', '168164', 'Raisins, golden, seedless' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Golden Raisins');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Honey Roasted Peanuts', 599, 0.609, 'usda', 'usda', '174261', 'Peanuts, all types, oil-roasted, with salt' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Honey Roasted Peanuts');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Mixed Nuts', 594, 0.579, 'usda', 'usda', '168599', 'Nuts, mixed nuts, dry roasted, with peanuts, with salt added' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Mixed Nuts');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Raisins', 299, 0.613, 'usda', 'usda', '168165', 'Raisins, dark, seedless' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Raisins');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Ramen Noodles', 440, NULL, NULL, 'usda', '171177', 'Soup, ramen noodle, any flavor, dry' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Ramen Noodles');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Red pepper flakes', 318, 0.358, 'usda', 'usda', '170932', 'Spices, pepper, red or cayenne' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Red pepper flakes');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Rice vinegar', 18, 1.006, 'usda', 'usda', '172237', 'Vinegar, distilled' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Rice vinegar');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Rolled Oats', 379, 0.342, 'usda', 'usda', '173904', 'Cereals, oats, regular and quick, not fortified, dry' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Rolled Oats');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Sesame oil', 884, 0.921, 'usda', 'usda', '171016', 'Oil, sesame, salad or cooking' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Sesame oil');
--> statement-breakpoint
INSERT INTO ingredient (user_id, name, kcal_per_100g, density_g_ml, density_source, source, source_id, reference_name) SELECT 1, 'Vanilla Granola', 489, 0.516, 'usda', 'usda', '171646', 'Cereals ready-to-eat, granola, homemade' WHERE EXISTS (SELECT 1 FROM meal WHERE user_id = 1 AND name = 'Oatmeal Bomb') AND NOT EXISTS (SELECT 1 FROM ingredient WHERE user_id = 1 AND name = 'Vanilla Granola');
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Chex Mix' AND i.name = 'Chex Mix';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Coconut Strips' AND i.name = 'Coconut Strips';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Crushed Nuts' AND i.name = 'Crushed Nuts';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Dried Cranberries' AND i.name = 'Dried Cranberries';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Golden Raisins' AND i.name = 'Golden Raisins';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Honey Roasted Peanuts' AND i.name = 'Honey Roasted Peanuts';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Mixed Nuts' AND i.name = 'Mixed Nuts';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Raisins' AND i.name = 'Raisins';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Ramen Noodles' AND i.name = 'Ramen Noodles';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Red pepper flakes' AND i.name = 'Red pepper flakes';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Rice vinegar' AND i.name = 'Rice vinegar';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Rolled Oats' AND i.name = 'Rolled Oats';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Sesame oil' AND i.name = 'Sesame oil';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Vanilla Granola' AND i.name = 'Vanilla Granola';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Brown sugar' AND i.name = 'BROWN SUGAR';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Peanut Butter' AND i.name = 'Peanut butter';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Soy sauce' AND i.name = 'Soy Sauce';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Dried minced onion' AND i.name = 'Minced Onion';
--> statement-breakpoint
UPDATE meal_ingredient mi SET ingredient_id = i.id FROM ingredient i, meal m WHERE mi.meal_id = m.id AND m.user_id = 1 AND i.user_id = 1 AND mi.ingredient_id IS NULL AND mi.snapshot_name = 'Powdered Milk' AND i.name = 'Dry milk';
