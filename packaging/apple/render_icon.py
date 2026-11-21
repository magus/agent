import pathlib
import subprocess


def render_icon():
    repo_dir = pathlib.Path(__file__).resolve().parents[2]
    version = "0.45.1"
    tool_dir = repo_dir / ".build/tools" / f"resvg-{version}"
    executable = tool_dir / "bin/resvg"
    if not executable.exists():
        subprocess.run(
            [
                "cargo", "install", "resvg", "--version", version, "--locked",
                "--root", str(tool_dir),
            ],
            check=True,
        )

    asset_dir = repo_dir / "assets/Agent.icon/Assets"
    subprocess.run(
        [
            str(executable), "--width", "1024", "--skip-system-fonts",
            str(asset_dir / "agent-icon-square.svg"),
            str(asset_dir / "agent-icon-square.png"),
        ],
        check=True,
    )


if __name__ == "__main__":
    render_icon()
