---
description: Prevents Python UnicodeDecodeError on Windows by enforcing UTF-8 encoding in subprocesses.
---

# Python Subprocess Encoding on Windows

Whenever writing Python scripts that spawn processes (via `subprocess.Popen` or `subprocess.run`) and decode text (`text=True` or `universal_newlines=True`), you **must explicitly declare** `encoding='utf-8'` and `errors='replace'`.

Windows uses `cp1252` by default for Python standard output decoding, which violently crashes with `UnicodeDecodeError` when Docker, Node, or Rust spits out UTF-8 formatting characters like emojis, progress bars, or box edges.

### Incorrect (Will crash randomly)
```python
process = subprocess.Popen(cmd, stdout=subprocess.PIPE, text=True)
result = subprocess.run(cmd, capture_output=True, text=True)
```

### Correct (Safe on all identical platforms)
```python
process = subprocess.Popen(cmd, stdout=subprocess.PIPE, text=True, encoding='utf-8', errors='replace')
result = subprocess.run(cmd, capture_output=True, text=True, encoding='utf-8', errors='replace')
```
