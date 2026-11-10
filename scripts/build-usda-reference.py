#!/usr/bin/env python3
"""Build the bundled USDA reference index from published SR and FNDDS CSVs."""
import csv
import json
import re
import sys
from fractions import Fraction
from pathlib import Path

ML = {'cup': 236.588, 'floz': 29.5735, 'tbsp': 14.7868, 'tsp': 4.92892}
UNITS = [('cup', r'\bcups?\b'), ('floz', r'\bfl\s+oz\b'),
         ('tbsp', r'\b(?:tablespoons?|tbsp)\b'), ('tsp', r'\b(?:teaspoons?|tsp)\b')]


def rows(path):
    with path.open(newline='', encoding='utf-8-sig') as stream:
        yield from csv.DictReader(stream)


def build(root):
    foods = {}
    ranks = {}
    for path in sorted(root.glob('*/*/food.csv')):
        for row in rows(path):
            if row['data_type'] not in ('sr_legacy_food', 'survey_fndds_food'):
                continue
            key = int(row['fdc_id'])
            description = re.sub(r"\s*\(Includes foods for USDA's.*\)\s*$", '', row['description']).strip()
            foods[key] = [key, description, None, None, None, 'sr' if row['data_type'] == 'sr_legacy_food' else 'fn']
        for row in rows(path.with_name('food_nutrient.csv')):
            food = foods.get(int(row['fdc_id']))
            if food and row['nutrient_id'] == ('1008' if food[5] == 'sr' else '208') and row['amount']:
                food[3] = int(float(row['amount']) + .5)
        for row in rows(path.with_name('food_portion.csv')):
            key = int(row['fdc_id'])
            food = foods.get(key)
            if not food:
                continue
            survey = food[5] == 'fn'
            label = row['portion_description'] if survey else row['modifier']
            if re.search(r'\b(?:unpacked|ice|guideline|cans?|bottles?|box(?:es)?|packets?|containers?' + ('|cooked' if survey else '') + r')\b', label, re.I):
                continue
            for rank, (unit, pattern) in enumerate(UNITS):
                if re.search(pattern, label, re.I):
                    break
            else:
                continue
            if key in ranks and ranks[key] <= rank:
                continue
            try:
                if survey:
                    match = re.match(r'^\s*(\d+\s+\d+/\d+|\d+/\d+|\d+(?:\.\d+)?)', label)
                    qty = sum(float(Fraction(p)) for p in match[1].split()) if match else 1
                else:
                    qty = float(row['amount'])
                density = round(float(row['gram_weight']) / (qty * ML[unit]), 3)
            except (ValueError, ZeroDivisionError):
                continue
            if .02 <= density <= 2.5:
                food[2], food[4], ranks[key] = density, unit, rank
    data = [foods[key] for key in sorted(foods)]
    target = Path(__file__).resolve().parents[1] / 'lib/usda-reference-data.json'
    target.write_text(json.dumps(data, separators=(',', ':'), ensure_ascii=False) + '\n')
    print(f'{len(data)} foods; {sum(food[2] is not None for food in data)} foods with density')
    for key in (171413, 168833, 170876, 173904):
        print(foods[key])


if __name__ == '__main__':
    build(Path(sys.argv[1]))
