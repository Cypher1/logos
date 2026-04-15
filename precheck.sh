set -xe
poetry run ty check logos
poetry run ruff check --fix logos
poetry run ruff format logos
