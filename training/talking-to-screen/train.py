#!/usr/bin/env python3
"""Train a small talking-to-the-screen classifier from labeled feature rows.

This does not download a dataset and it does not report a real accuracy unless
you pass a JSONL file you recorded. `--smoke` fits a separable toy set so the
script can be checked on a CPU. That run always prints dataset=synthetic-smoke
and is not a claim about saywhen users.

Each JSONL row is one moment:

    {"yaw": 2.1, "pitch": -1.0, "mar": 0.04, "mar_variance": 0.00002,
     "jaw_open": 0.2, "smile": 0.05, "mouth_close": 0.1, "vad": 0.8,
     "label": 1}

label 1 means the person is talking to the screen. 0 means they are not.
The features are the same numbers the app already computes in memory.

On a RunPod GPU pod the logistic model is still small enough for a CPU, which
is the point: collect the rows on a desk, then fit them anywhere.

    python3 training/talking-to-screen/train.py data/session.jsonl --out /tmp/talking.json
    python3 training/talking-to-screen/train.py --smoke
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

FEATURES = (
    "yaw",
    "pitch",
    "mar",
    "mar_variance",
    "jaw_open",
    "smile",
    "mouth_close",
    "vad",
)


def load_rows(path: Path) -> list[dict]:
    rows = []
    for line_number, line in enumerate(path.read_text().splitlines(), start=1):
        text = line.strip()
        if not text:
            continue
        row = json.loads(text)
        if "label" not in row:
            raise SystemExit(f"{path}:{line_number} is missing label")
        for name in FEATURES:
            if name not in row:
                raise SystemExit(f"{path}:{line_number} is missing {name}")
        rows.append(row)
    if len(rows) < 8:
        raise SystemExit(f"{path} needs at least 8 labeled rows, found {len(rows)}")
    return rows


def smoke_rows() -> list[dict]:
    rows = []
    for index in range(40):
        talking = index % 2 == 0
        rows.append(
            {
                "yaw": 4.0 if talking else 40.0,
                "pitch": 1.0 if talking else 8.0,
                "mar": 0.08 if talking else 0.012,
                "mar_variance": 0.0004 if talking else 0.000001,
                "jaw_open": 0.4 if talking else 0.02,
                "smile": 0.05,
                "mouth_close": 0.05 if talking else 0.4,
                "vad": 0.9 if talking else 0.05,
                "label": 1 if talking else 0,
            }
        )
    return rows


def standardize(rows: list[dict]) -> tuple[list[list[float]], list[float], list[float]]:
    matrix = [[float(row[name]) for name in FEATURES] for row in rows]
    means = []
    scales = []
    for column in range(len(FEATURES)):
        values = [row[column] for row in matrix]
        mean = sum(values) / len(values)
        variance = sum((value - mean) ** 2 for value in values) / len(values)
        scale = math.sqrt(variance) or 1.0
        means.append(mean)
        scales.append(scale)
        for row in matrix:
            row[column] = (row[column] - mean) / scale
    return matrix, means, scales


def fit(matrix: list[list[float]], labels: list[int], steps: int = 400, rate: float = 0.2) -> list[float]:
    weights = [0.0] * (len(FEATURES) + 1)
    for _ in range(steps):
        gradients = [0.0] * len(weights)
        for row, label in zip(matrix, labels):
            score = weights[0] + sum(weight * value for weight, value in zip(weights[1:], row))
            prediction = 1.0 / (1.0 + math.exp(-max(min(score, 30.0), -30.0)))
            error = prediction - label
            gradients[0] += error
            for index, value in enumerate(row):
                gradients[index + 1] += error * value
        scale = len(labels)
        weights = [weight - rate * (gradient / scale) for weight, gradient in zip(weights, gradients)]
    return weights


def predict(weights: list[float], row: list[float]) -> int:
    score = weights[0] + sum(weight * value for weight, value in zip(weights[1:], row))
    return 1 if score >= 0 else 0


def metrics(weights: list[float], matrix: list[list[float]], labels: list[int]) -> dict[str, float]:
    correct = 0
    true_positive = 0
    false_positive = 0
    false_negative = 0
    for row, label in zip(matrix, labels):
        guess = predict(weights, row)
        correct += int(guess == label)
        true_positive += int(guess == 1 and label == 1)
        false_positive += int(guess == 1 and label == 0)
        false_negative += int(guess == 0 and label == 1)
    precision = true_positive / (true_positive + false_positive) if true_positive + false_positive else 0.0
    recall = true_positive / (true_positive + false_negative) if true_positive + false_negative else 0.0
    return {
        "accuracy": correct / len(labels),
        "precision": precision,
        "recall": recall,
        "rows": float(len(labels)),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dataset", nargs="?", help="JSONL file of labeled feature rows")
    parser.add_argument("--smoke", action="store_true", help="Fit a toy set. Not a real result.")
    parser.add_argument("--out", help="Where to write weights JSON")
    args = parser.parse_args()
    if args.smoke == bool(args.dataset):
        parser.error("Pass a JSONL dataset or --smoke, not both and not neither")
    if args.smoke:
        rows = smoke_rows()
        dataset_name = "synthetic-smoke"
    else:
        rows = load_rows(Path(args.dataset))
        dataset_name = str(args.dataset)
    labels = [1 if int(row["label"]) == 1 else 0 for row in rows]
    if len(set(labels)) < 2:
        raise SystemExit("The dataset needs both label 0 and label 1")
    matrix, means, scales = standardize(rows)
    weights = fit(matrix, labels)
    report = metrics(weights, matrix, labels)
    report_out = {
        "dataset": dataset_name,
        "claim": "smoke-only" if args.smoke else "measured-on-this-file",
        "features": list(FEATURES),
        "weights": weights,
        "means": means,
        "scales": scales,
        "metrics": report,
        "note": (
            "Synthetic smoke test. Do not quote this accuracy."
            if args.smoke
            else "Scores are on the file you passed, not on held-out users, unless you split it yourself."
        ),
    }
    text = json.dumps(report_out, indent=2)
    if args.out:
        Path(args.out).write_text(text + "\n")
    print(text)
    if args.smoke and report["accuracy"] < 0.95:
        print("smoke fit failed to separate the toy set", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
