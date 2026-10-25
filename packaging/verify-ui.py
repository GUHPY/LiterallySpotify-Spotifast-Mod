"""Run the focused UI/update regression gate and reject missing tests."""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    tests = [line.strip() for line in (ROOT / "packaging/ui-regression-tests.txt").read_text(encoding="utf-8").splitlines() if line.strip() and not line.startswith("#")]
    if not tests or len(set(tests)) != len(tests):
        raise ValueError("The regression list is empty or contains duplicate tests")
    log = ROOT / "target/ui-regression-tests.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("wb") as output:
        result = subprocess.run(["cargo", "test", "--locked", "--all-targets", "--", "--exact", *tests], cwd=ROOT, stdout=output, stderr=subprocess.STDOUT)
    text = log.read_text(encoding="utf-8", errors="replace")
    print(text)
    passed = sum(int(count) for count in re.findall(r"test result: ok\. (\d+) passed;", text))
    if result.returncode or passed != len(tests):
        raise RuntimeError(f"Regression gate failed: {passed}/{len(tests)} tests passed")
    print(f"All {passed} declared UI/update regression tests passed.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
