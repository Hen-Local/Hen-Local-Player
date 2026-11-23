"""Expand project headers for source snippets compiled by the Metal runtime."""
import re
import sys
from pathlib import Path


def expand(repo, path):
    name = path.relative_to(repo).as_posix()
    guard = "MLX_JIT_HEADER_" + re.sub(r"[^a-zA-Z0-9]", "_", name).upper()
    lines = [f"#ifndef {guard}", f"#define {guard}", f'#line 1 "{name}"']
    for line_number, line in enumerate(path.read_text().splitlines(), 1):
        include = re.match(r'\s*#\s*include\s*"([^"]+)"', line)
        if include:
            target = path.parent / include[1]
            if not target.is_file():
                target = repo / include[1]
            lines.extend([expand(repo, target), f'#line {line_number + 1} "{name}"'])
        elif line.strip() != "#pragma once":
            lines.append(line)
    return "\n".join(lines + ["#endif"])


if __name__ == "__main__":
    output, _, source, kernel, *_ = sys.argv[1:]
    repo = Path(source)
    entry = repo / "mlx/backend/metal/kernels" / (kernel + ".h")
    name = entry.stem
    body = expand(repo, entry)
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    (output / (name + ".cpp")).write_text(
        f'namespace mlx::core::metal {{\nconst char* {name}() {{ return R"preamble(\n'
        f'{body}\n)preamble"; }}\n}}\n'
    )
