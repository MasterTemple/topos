Make sure you have uv and maturin installed, and a virtual env

First time
```bash
python -m venv .env
source .env/bin/activate
pip install maturin
```

```bash
maturin develop
```

Each time you open a terminal, run

```bash
source .env/bin/activate
```

Then just run the python file like normal

Additional help: https://pyo3.rs/v0.25.1/getting-started.html

Usage

```py
import topos
```
