# Native icon origin

The PNG and its ICO/ICNS containers are generated locally by [create_icons.py](../../../../scripts/create_icons.py). The source draws a neutral bidirectional transfer mark using the application's warm paper and evergreen colors. No external artwork or image model is used.

Run `python3 scripts/create_icons.py` from the repository root to reproduce all three files. The generated PNG includes an `impeccable:prompt` text chunk containing its code origin; ICO/ICNS contain that same PNG. Metadata/provenance was checked without displaying or inspecting images, as `AGENTS.md` requires.
