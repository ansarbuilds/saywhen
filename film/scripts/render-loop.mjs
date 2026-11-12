import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const film = fileURLToPath(new URL("../", import.meta.url));
const destination = fileURLToPath(new URL("../../docs/visuals/animated/", import.meta.url));
const specs = JSON.parse(readFileSync(join(film, "src/loop-specs.json"), "utf8"));
const spec = specs.find(({ name }) => name === process.argv[2]);
const [, , , option, frameDirectory] = process.argv;
if (!spec || (option && (option !== "--frames-dir" || !frameDirectory))) {
  throw new Error("Usage: node scripts/render-loop.mjs <name> [--frames-dir <directory>]");
}

const out = join(film, "out");
mkdirSync(out, { recursive: true });
const scratch = mkdtempSync(join(out, `${spec.name}-`));
const remotion = join(film, "node_modules/.bin/remotion");
const source = join(scratch, "source.mp4");
const palette = join(scratch, "palette.png");

function run(command, args) {
  execFileSync(command, args, { cwd: film, stdio: "inherit" });
}

function ffmpeg(args) {
  run("ffmpeg", ["-hide_banner", "-loglevel", "error", "-y", ...args]);
}

// Repeat the opening image at the seam before the final encode.
function closeLoop(filter, fps) {
  return [
    `[0:v]${filter},split[run][start]`,
    `[run]trim=end_frame=${spec.duration * fps - 1}[body]`,
    "[start]trim=end_frame=1[opening]",
    `[body][opening]concat=n=2:v=1:a=0,setpts=N/(${fps}*TB),fps=${fps}[loop]`,
  ].join(";");
}

try {
  const fps = 20;
  let input = ["-i", source];
  if (frameDirectory) {
    const directory = resolve(frameDirectory);
    for (let frame = 0; frame < spec.duration * fps; frame++) {
      statSync(join(directory, `${String(frame).padStart(4, "0")}.png`));
    }
    input = ["-framerate", String(fps), "-i", join(directory, "%04d.png")];
  } else {
    run(remotion, [
      "render", spec.name, source, "--codec=h264", "--crf=12", "--muted", "--concurrency=3",
    ]);
  }
  const gif = `${spec.name}.gif`;
  const mp4 = `${spec.name}.mp4`;
  const png = `${spec.name}.png`;
  const gifScale = "fps=10:start_time=0,scale=800:-2:flags=lanczos";
  ffmpeg([...input, "-t", String(spec.duration), "-vf", `${gifScale},palettegen=stats_mode=full`, "-frames:v", "1", palette]);
  ffmpeg([
    ...input, "-i", palette, "-t", String(spec.duration), "-filter_complex",
    `${closeLoop(gifScale, 10)};[loop][1:v]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle`,
    "-loop", "0", "-final_delay", "10", join(scratch, gif),
  ]);
  ffmpeg([
    ...input, "-t", String(spec.duration), "-filter_complex", closeLoop("scale=1280:-2:flags=lanczos", fps),
    "-map", "[loop]", "-an", "-c:v", "libx264", "-preset", "slow", "-qp", "20",
    "-force_key_frames", String(spec.duration - 1 / fps), "-forced-idr", "1",
    "-pix_fmt", "yuv420p", "-movflags", "+faststart", join(scratch, mp4),
  ]);
  ffmpeg([
    ...input, "-ss", String(spec.posterAt), "-vf", "scale=1280:-2:flags=lanczos",
    "-frames:v", "1", "-update", "1", join(scratch, png),
  ]);
  for (const [file, limit] of [[gif, 2_500_000], [mp4, 1_500_000]]) {
    if (statSync(join(scratch, file)).size >= limit) throw new Error(`${file} exceeds ${limit} bytes.`);
  }
  mkdirSync(destination, { recursive: true });
  for (const file of [gif, mp4, png]) renameSync(join(scratch, file), join(destination, file));
  const manifest = specs.map(({ name, width, height, duration, alt, caption, placement }) => ({
    name, width, height, duration, alt, caption, placement,
    files: Object.fromEntries([["gif", 800], ["mp4", 1280], ["png", 1280]].map(([format, fileWidth]) => [
      format, { name: `${name}.${format}`, width: fileWidth, height: height * fileWidth / width },
    ])),
  }));
  writeFileSync(join(scratch, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  renameSync(join(scratch, "manifest.json"), join(destination, "manifest.json"));
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
