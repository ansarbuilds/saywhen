# Talking-to-the-screen classifier

saywhen already decides "talking to the screen" with head angle, lip shape, and the microphone. The open question in the architecture notes is whether a small learned model would separate speech from chewing, smiling, and looking away better than those rules.

This folder trains that model. It does not ship weights, and it does not invent an accuracy number.

## What you record

One JSON line per moment, using numbers the app already computes and then throws away with the camera frame:

- `yaw`, `pitch`: head angle in degrees
- `mar`, `mar_variance`: mouth opening and how much it is moving
- `jaw_open`, `smile`, `mouth_close`: blendshape scores
- `vad`: microphone voice score from 0 to 1
- `label`: `1` when you were talking to the screen, `0` when you were not

Do not put pictures or audio in the file. A few hundred rows of you talking, chewing, smiling, and looking away is enough to see if the model beats the current rules on your desk. Keep a second file the trainer never sees, and score that file separately before trusting the number.

## Run it

On a laptop, or on a RunPod pod (a CPU is enough; this is logistic regression, not a big network):

```bash
python3 training/talking-to-screen/train.py /path/to/labeled.jsonl --out /tmp/talking.json
```

Check the script itself, with a toy set that is not a result:

```bash
python3 training/talking-to-screen/train.py --smoke
```

`--smoke` prints `"dataset": "synthetic-smoke"` and `"claim": "smoke-only"`. Do not quote that accuracy.

## After a real run

If the held-out file beats the current rules by a margin you can name, a follow-up change can load the weights inside the vision gate. Until that file exists, the app keeps the rules it has.
