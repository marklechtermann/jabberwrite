// Generates ACKNOWLEDGEMENTS.md, the single file with all third-party licenses (also shown in Settings > Licenses),
// and checks every license against the allow-list below (compatibility with our Apache-2.0 license, ADR 0006).
// Run `npm run licenses` after every change to Cargo.lock or package-lock.json; CI fails if the file is stale.
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "ACKNOWLEDGEMENTS.md");
const thirdParty = join(root, "third-party");

// Permissive licenses (plus file-level MPL-2.0 for unmodified dependencies) that may be shipped inside an
// Apache-2.0 application. An SPDX expression passes when one OR-alternative is fully allowed.
// Anything else (GPL, LGPL-only, AGPL, unknown, ...) fails the run and needs a decision from the maintainer.
const allowed = new Set([
  "Apache-2.0",
  "MIT",
  "MIT-0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "0BSD",
  "Unlicense",
  "CC0-1.0",
  "BSL-1.0",
  "bzip2-1.0.6",
  "MPL-2.0",
  "Unicode-3.0",
  "Unicode-DFS-2016",
  "CDLA-Permissive-2.0",
]);

// Components that are not Cargo or npm packages: native libraries statically linked or bundled by
// sherpa-onnx (versions from its release v2.0.1) and the speech model downloaded at runtime.
// Add an entry here when a dependency ships outside of Cargo.lock and package-lock.json.
// `files` are license texts from third-party/.
const manual = [
  {
    name: "ONNX Runtime (native, via sherpa-onnx)",
    version: "1.28.2",
    license: "MIT",
    url: "https://github.com/microsoft/onnxruntime",
    files: ["onnxruntime-LICENSE.txt", "onnxruntime-ThirdPartyNotices.txt"],
  },
  {
    name: "sherpa-onnx (native libraries)",
    version: "2.0.1",
    license: "Apache-2.0",
    url: "https://github.com/k2-fsa/sherpa-onnx",
    files: ["Apache-2.0.txt"],
  },
  {
    name: "kaldi-decoder",
    version: "0.3.0",
    license: "Apache-2.0",
    url: "https://github.com/k2-fsa/kaldi-decoder",
    files: ["Apache-2.0.txt"],
  },
  {
    name: "kaldi-native-fbank",
    version: "1.22.4",
    license: "Apache-2.0",
    url: "https://github.com/csukuangfj/kaldi-native-fbank",
    files: ["Apache-2.0.txt"],
  },
  {
    name: "OpenFst",
    version: "1.8.5",
    license: "Apache-2.0",
    url: "https://github.com/csukuangfj/openfst",
    files: ["openfst-COPYING.txt"],
  },
  {
    name: "simple-sentencepiece",
    version: "0.7",
    license: "Apache-2.0",
    url: "https://github.com/pkufool/simple-sentencepiece",
    files: ["Apache-2.0.txt"],
  },
  {
    name: "KISS FFT",
    version: "febd4ca",
    license: "BSD-3-Clause",
    url: "https://github.com/mborgerding/kissfft",
    files: ["kissfft-COPYING.txt"],
  },
  {
    name: "utf8proc",
    version: "2.12.0",
    license: "MIT",
    url: "https://github.com/JuliaStrings/utf8proc",
    files: ["utf8proc-LICENSE.md"],
  },
  {
    name: "Orukeet speech recognition model (downloaded at runtime, not distributed with the app)",
    version: "",
    license: "CC-BY-SA-4.0",
    url: "https://creativecommons.org/licenses/by-sa/4.0/",
    files: ["CC-BY-SA-4.0.txt"],
  },
  {
    name: "NVIDIA Parakeet TDT 0.6B v3 (base of the Orukeet model)",
    version: "",
    license: "CC-BY-4.0",
    url: "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3",
    files: ["CC-BY-4.0.txt"],
  },
];

const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
const packages = [];
const violations = [];

const normalize = (text) => text.replace(/\r\n/g, "\n").trim();

function licenseFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isFile() && /^(licen[cs]e|copying|notice|unlicense)/i.test(e.name))
    .sort((a, b) => cmp(a.name, b.name))
    .map((e) => normalize(readFileSync(join(dir, e.name), "utf8")))
    .filter(Boolean);
}

// Evaluates an SPDX expression against `allowed`.
function isAllowed(expression) {
  const tokens = expression.replace(/\//g, " OR ").match(/\(|\)|[^\s()]+/g) ?? [];
  let pos = 0;
  const atom = () => {
    if (tokens[pos] === "(") {
      pos++;
      const value = or();
      pos++;
      return value;
    }
    const id = tokens[pos++].replace(/\+$/, "");
    if (tokens[pos] === "WITH") pos += 2;
    return allowed.has(id);
  };
  const and = () => {
    let value = atom();
    while (tokens[pos] === "AND") {
      pos++;
      value = atom() && value;
    }
    return value;
  };
  const or = () => {
    let value = and();
    while (tokens[pos] === "OR") {
      pos++;
      value = and() || value;
    }
    return value;
  };
  return or();
}

const identifiers = (expression) =>
  (expression.match(/[A-Za-z0-9.\-]+/g) ?? []).filter((t) => !["OR", "AND", "WITH"].includes(t));

// Some crates ship no license file. Use the standard text of one of the licenses they offer.
function fallbackTexts(name, license, authors) {
  const ids = identifiers(license);
  const id = ["MIT", "BSD-3-Clause", "BSL-1.0", "MPL-2.0"].find((t) => ids.includes(t));
  if (id) {
    const holders = authors.map((a) => a.replace(/\s*<[^>]*>/, "")).filter(Boolean);
    const copyright = `Copyright (c) ${holders.length ? holders.join(", ") : `the ${name} authors`}`;
    return [normalize(readFileSync(join(thirdParty, `template-${id}.txt`), "utf8")).replace("{copyright}", copyright)];
  }
  if (ids.includes("Apache-2.0")) return [normalize(readFileSync(join(thirdParty, "Apache-2.0.txt"), "utf8"))];
  return [];
}

// Cargo: everything reachable from the app through normal (shipped) dependencies.
const metadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--format-version", "1", "--locked"], {
    cwd: join(root, "src-tauri"),
    maxBuffer: 1 << 30,
    encoding: "utf8",
  }),
);
const byId = new Map(metadata.packages.map((p) => [p.id, p]));
const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
const seen = new Set();
const queue = [metadata.resolve.root];
while (queue.length) {
  const id = queue.pop();
  if (seen.has(id)) continue;
  seen.add(id);
  for (const dep of nodes.get(id).deps) {
    if (dep.dep_kinds.some((k) => k.kind === null)) queue.push(dep.pkg);
  }
}
seen.delete(metadata.resolve.root);
for (const id of seen) {
  const p = byId.get(id);
  const license = p.license ?? "UNKNOWN";
  if (!isAllowed(license)) violations.push(`${p.name} ${p.version}: ${license}`);
  const files = licenseFiles(dirname(p.manifest_path));
  packages.push({
    name: p.name,
    version: p.version,
    license,
    url: p.repository ?? p.homepage ?? `https://crates.io/crates/${p.name}`,
    generated: files.length === 0,
    texts: files.length ? files : fallbackTexts(p.name, license, p.authors),
  });
}

// npm: production packages from the lockfile (dev tools are not shipped).
const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
for (const [path, p] of Object.entries(lock.packages)) {
  if (!path || p.dev) continue;
  const name = path.slice(path.lastIndexOf("node_modules/") + "node_modules/".length);
  const license = p.license ?? "UNKNOWN";
  if (!isAllowed(license)) violations.push(`${name} ${p.version}: ${license}`);
  const files = licenseFiles(join(root, path));
  packages.push({
    name,
    version: p.version,
    license,
    url: `https://www.npmjs.com/package/${name}`,
    generated: files.length === 0,
    texts: files.length ? files : fallbackTexts(name, license, []),
  });
}

for (const m of manual) {
  const { files, ...entry } = m;
  packages.push({ ...entry, generated: false, texts: files.map((f) => normalize(readFileSync(join(thirdParty, f), "utf8"))) });
}

if (violations.length) {
  console.error(`Licenses not on the allow-list (see scripts/generate-licenses.mjs):\n  ${violations.join("\n  ")}`);
  process.exit(1);
}

const missing = packages.filter((p) => p.texts.length === 0);
if (missing.length) {
  console.error(`No license text for:\n  ${missing.map((p) => `${p.name} ${p.version} (${p.license})`).join("\n  ")}`);
  process.exit(1);
}

// One section per distinct license text, listing every component it applies to. Sections are headed
// "## <license type> | <copyright line> | <count>" and sorted by type. Text lines are indented by four spaces,
// so lines at column 0 are structure; the Licenses tab parses this format.
const kinds = [
  ["Apache-2.0", /Apache License\s+Version 2\.0/i],
  ["MPL-2.0", /Mozilla Public License Version 2\.0/],
  ["BSL-1.0", /Boost Software License/],
  ["CC-BY-SA-4.0", /^Attribution-ShareAlike 4\.0/],
  ["CC-BY-4.0", /^Attribution 4\.0/],
  ["Unicode-3.0", /UNICODE LICENSE V3/i],
  ["Unlicense", /free and\s+unencumbered\s+software/],
  ["CC0-1.0", /CC0 1\.0 Universal/],
  ["CDLA-Permissive-2.0", /Community Data License Agreement - Permissive/],
  ["ISC", /Permission to use, copy, modify, and\/or distribute/],
  ["Zlib", /provided 'as-is'/],
  ["BSD-3-Clause", /Redistribution and use[\s\S]*Neither the name/],
  ["BSD-2-Clause", /Redistribution and use/],
  ["MIT", /Permission is hereby granted, free of charge/],
];
const copyrightLine = /^\s*(copyright|\(c\))/i;

function describe(text) {
  if (text.length > 50000) return { kind: "Third-party notices", detail: "" };
  const kind = kinds.find(([, re]) => re.test(text))?.[0] ?? "Other";
  const long = ["Apache-2.0", "MPL-2.0", "BSL-1.0", "CC-BY-SA-4.0", "CC-BY-4.0", "CC0-1.0", "CDLA-Permissive-2.0"].includes(kind);
  const line = long
    ? undefined
    : kind === "Other"
      ? text.split("\n").find((l) => l.trim())
      : text.split("\n").find((l) => copyrightLine.test(l));
  return { kind, detail: (line ?? "").trim().slice(0, 100).replace(/\s+/g, " ") };
}

const groups = new Map();
for (const p of packages) {
  for (const text of p.texts) {
    if (!groups.has(text)) groups.set(text, []);
    groups.get(text).push(p);
  }
}
const sections = [...groups]
  .map(([text, members]) => ({
    text,
    ...describe(text),
    members: members.sort((a, b) => cmp(a.name, b.name) || cmp(a.version, b.version)),
  }))
  .sort((a, b) => cmp(a.kind, b.kind) || cmp(a.detail, b.detail) || cmp(a.members[0].name, b.members[0].name));

const lines = [
  "# Acknowledgements",
  "",
  "Jabberwrite is licensed under the Apache License 2.0 (see LICENSE). It includes the open source components listed below.",
  "Generated by `npm run licenses`; do not edit by hand.",
];
for (const { text, kind, detail, members } of sections) {
  lines.push("", `## ${kind} | ${detail} | ${members.length}`, "");
  for (const p of members) {
    const note = p.generated ? " | no license file shipped, standard text" : "";
    lines.push(`- ${`${p.name} ${p.version}`.trim()} | ${p.license} | ${p.url}${note}`);
  }
  lines.push("");
  for (const line of text.split("\n")) lines.push(line ? `    ${line}` : "");
}
writeFileSync(out, lines.join("\n") + "\n");
console.log(`${packages.length} components, ${sections.length} distinct license texts -> ${out}`);
