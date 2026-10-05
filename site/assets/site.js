// Sol JK site enhancements. The page works without this script: the download
// button points at the latest release page and the changelog links to GitHub.
"use strict";

const REPO = "Sol-Vulpes/SJK";
const RELEASES_API = `https://api.github.com/repos/${REPO}/releases?per_page=6`;
const RELEASES_PAGE = `https://github.com/${REPO}/releases`;

/** Escape text for safe insertion as HTML. */
function escapeHtml(text) {
  return String(text).replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[c]);
}

/** Inline markdown on already-escaped text: code, bold, links, bare URLs. */
function inline(escaped) {
  const codes = [];
  let text = escaped.replace(/`([^`]+)`/g, (_, code) => {
    codes.push(`<code>${code}</code>`);
    return `\u0000${codes.length - 1}\u0000`;
  });
  text = text
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/(^|[\s(])(https?:\/\/[^\s<)]+)/g, (_, lead, url) => {
      const label = url.replace(/^https:\/\/github\.com\/[^/]+\/[^/]+\/pull\//, "#");
      return `${lead}<a href="${url}">${label}</a>`;
    });
  return text.replace(/\u0000(\d+)\u0000/g, (_, i) => codes[Number(i)]);
}

/** The `<img>` tags of a raw HTML line, rebuilt from their https src, alt and width. */
function htmlImages(raw) {
  const images = [];
  for (const [tag] of raw.matchAll(/<img\b[^>]*>/gi)) {
    const attr = (name) => (tag.match(new RegExp(`\\s${name}\\s*=\\s*"([^"]*)"`, "i")) || [])[1];
    const src = attr("src");
    if (!src || !/^https:\/\//i.test(src)) continue;
    const width = /^\d+$/.test(attr("width") || "") ? ` width="${attr("width")}"` : "";
    images.push(`<img src="${escapeHtml(src)}" alt="${escapeHtml(attr("alt") || "")}"${width}>`);
  }
  return images;
}

/** A small markdown subset for release notes: headings, lists, paragraphs.
 * A line of raw HTML keeps only its images; any other markup is dropped. */
function renderMarkdown(source) {
  const out = [];
  let list = false;
  let paragraph = [];
  const flush = () => {
    if (paragraph.length) out.push(`<p>${inline(paragraph.join(" "))}</p>`);
    paragraph = [];
  };
  const closeList = () => {
    if (list) out.push("</ul>");
    list = false;
  };
  for (const raw of String(source || "").replace(/\r\n?/g, "\n").split("\n")) {
    if (raw.trim().startsWith("<")) {
      flush(); closeList();
      const images = htmlImages(raw);
      if (images.length) out.push(`<p class="release-image">${images.join(" ")}</p>`);
      continue;
    }
    const line = escapeHtml(raw.trimEnd());
    const heading = line.match(/^#{1,6}\s+(.*)$/);
    const item = line.match(/^\s*[-*]\s+(.*)$/);
    if (heading) {
      flush(); closeList();
      out.push(`<h4>${inline(heading[1])}</h4>`);
    } else if (item) {
      flush();
      if (!list) { out.push("<ul>"); list = true; }
      out.push(`<li>${inline(item[1])}</li>`);
    } else if (!line.trim()) {
      flush(); closeList();
    } else {
      closeList();
      paragraph.push(line.trim());
    }
  }
  flush(); closeList();
  return out.join("\n");
}

/** The newest published, non-prerelease release, or the newest of any kind. */
function latestRelease(releases) {
  const published = releases.filter((r) => !r.draft);
  return published.find((r) => !r.prerelease) || published[0] || null;
}

/** Download asset for a platform ("windows-x64" or "linux-x64"). */
function assetFor(release, platform) {
  return (release.assets || []).find((a) => a.name.endsWith(`-${platform}.zip`)) || null;
}

function versionOf(release) {
  return (release.tag_name || "").replace(/^sjk-v/, "") || release.name || "";
}

function guessPlatform() {
  const agent = `${navigator.userAgent} ${navigator.platform || ""}`;
  return /Linux|X11/i.test(agent) && !/Android/i.test(agent) ? "linux-x64" : "windows-x64";
}

function enhanceDownload(release) {
  const button = document.getElementById("download");
  const files = document.getElementById("release-files");
  if (!button || !release) return;
  const version = versionOf(release);
  const platform = guessPlatform();
  const asset = assetFor(release, platform);
  if (asset) {
    button.href = asset.browser_download_url;
    button.textContent = `Download ${version} for ${platform === "linux-x64" ? "Linux" : "Windows"}`;
  } else {
    button.href = release.html_url;
    button.textContent = `Download ${version}`;
  }
  if (files) {
    const links = ["windows-x64", "linux-x64"]
      .map((p) => [p, assetFor(release, p)])
      .filter(([, a]) => a)
      .map(([p, a]) => `<a href="${escapeHtml(a.browser_download_url)}">${p === "linux-x64" ? "Linux x64" : "Windows x64"}</a>`);
    links.push(`<a href="${escapeHtml(release.html_url)}">all files &amp; checksums</a>`);
    files.innerHTML = links.join(" · ");
    files.hidden = false;
  }
}

function renderReleases(releases) {
  const target = document.getElementById("releases");
  if (!target) return;
  const shown = releases.filter((r) => !r.draft).slice(0, 5);
  if (!shown.length) {
    target.innerHTML = `<p class="muted">No release yet — the first one is on its way.
      Meanwhile you can build SJK from source (see Install), or follow
      <a href="${RELEASES_PAGE}">the releases page</a>.</p>`;
    return;
  }
  target.innerHTML = shown.map((r) => {
    const date = r.published_at ? new Date(r.published_at) : null;
    const when = date ? `<time datetime="${escapeHtml(r.published_at)}">${date.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" })}</time>` : "";
    const title = escapeHtml(r.name || r.tag_name);
    return `<article class="panel release">
      <header><h3><a href="${escapeHtml(r.html_url)}">${title}</a></h3>${when}</header>
      <div class="release-body">${renderMarkdown(r.body)}</div>
    </article>`;
  }).join("\n") + `<p class="muted"><a href="${RELEASES_PAGE}">All releases</a></p>`;
}

async function loadReleases() {
  try {
    const response = await fetch(RELEASES_API, { headers: { Accept: "application/vnd.github+json" } });
    if (!response.ok) throw new Error(`GitHub API ${response.status}`);
    const releases = await response.json();
    if (!Array.isArray(releases)) throw new Error("unexpected response");
    renderReleases(releases);
    enhanceDownload(latestRelease(releases));
  } catch (error) {
    // Keep the static links; they always work.
    console.info("Release information unavailable:", error);
  }
}

async function loadGallery() {
  const section = document.getElementById("screenshots");
  const gallery = document.getElementById("gallery");
  const dialog = document.getElementById("lightbox");
  if (!section || !gallery) return;
  let shots;
  try {
    const response = await fetch("screenshots/manifest.json", { cache: "no-cache" });
    if (!response.ok) return;
    shots = (await response.json()).filter((s) => s && typeof s.file === "string");
  } catch {
    return;
  }
  if (!shots.length) return;
  gallery.innerHTML = shots.map((s, i) => `<button type="button" class="shot" data-index="${i}">
      <img src="screenshots/${encodeURI(s.file)}" alt="${escapeHtml(s.caption || "SJK screenshot")}" loading="lazy" decoding="async">
      ${s.caption ? `<span>${escapeHtml(s.caption)}</span>` : ""}
    </button>`).join("\n");
  section.hidden = false;
  document.querySelectorAll("[data-gallery-link]").forEach((el) => { el.hidden = false; });

  if (!dialog || typeof dialog.showModal !== "function") {
    gallery.addEventListener("click", (event) => {
      const button = event.target.closest(".shot");
      if (button) window.open(`screenshots/${encodeURI(shots[Number(button.dataset.index)].file)}`, "_blank", "noopener");
    });
    return;
  }
  const image = document.getElementById("lightbox-image");
  const caption = document.getElementById("lightbox-caption");
  let current = 0;
  const show = (index) => {
    current = (index + shots.length) % shots.length;
    image.src = `screenshots/${encodeURI(shots[current].file)}`;
    image.alt = shots[current].caption || "SJK screenshot";
    caption.textContent = shots[current].caption || "";
  };
  gallery.addEventListener("click", (event) => {
    const button = event.target.closest(".shot");
    if (!button) return;
    show(Number(button.dataset.index));
    dialog.showModal();
  });
  dialog.addEventListener("click", (event) => {
    const step = event.target.closest("[data-step]");
    if (step) show(current + Number(step.dataset.step));
    else if (event.target.closest("[data-close]") || event.target === dialog) dialog.close();
  });
  dialog.addEventListener("keydown", (event) => {
    if (event.key === "ArrowRight") show(current + 1);
    if (event.key === "ArrowLeft") show(current - 1);
  });
}

if (typeof document !== "undefined") {
  document.addEventListener("DOMContentLoaded", () => {
    loadReleases();
    loadGallery();
  });
}

if (typeof module !== "undefined") {
  module.exports = { escapeHtml, inline, renderMarkdown, latestRelease, assetFor, versionOf };
}
