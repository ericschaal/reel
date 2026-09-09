const movie = {
  title: "Sintel",
  year: 2010,
  runtime: "15 min",
  rating: "PG",
  genres: ["Animation", "Fantasy"],
  synopsis:
    "A solitary warrior searches a frozen world for the dragon she lost long ago.",
};

const sources = [
  { id: "jf", kind: "Jellyfin", label: "Local library", detail: "Not available", disabled: true },
  { id: "s1", kind: "Stremio · 1", label: "4K · HDR · 5.1", detail: "12.4 GB" },
  { id: "s2", kind: "Stremio · 2", label: "1080p · H.264 · 5.1", detail: "3.8 GB" },
  { id: "s3", kind: "Stremio · 3", label: "720p · H.264 · Stereo", detail: "1.2 GB" },
];

const variants = [
  { key: "A", name: "Cinematic reveal" },
  { key: "B", name: "Decision first" },
  { key: "C", name: "Compact source sheet" },
];

const state = {
  view: "catalog",
  sourcesDiscovered: false,
  sourcesVisible: false,
  activation: "idle",
  selectedSource: null,
  message: "Select the movie to inspect playback options.",
};

const app = document.querySelector("#app");
const stateElement = document.querySelector("#state");
const variantLabel = document.querySelector("#variant-label");

function currentVariant() {
  const key = new URLSearchParams(window.location.search).get("variant")?.toUpperCase();
  return variants.find((variant) => variant.key === key) ?? variants[0];
}

function posterMarkup() {
  return `
    <div class="poster-art" aria-label="Abstract artwork for ${movie.title}">
      <span class="poster-moon"></span>
      <span class="poster-mountain one"></span>
      <span class="poster-mountain two"></span>
      <strong>${movie.title}</strong>
    </div>`;
}

function sourceRows({ selectable = true } = {}) {
  return sources
    .map(
      (source, index) => `
        <button class="source-row ${source.disabled ? "disabled" : ""} ${state.selectedSource === source.id ? "selected" : ""}"
          type="button" ${source.disabled || !selectable ? "disabled" : ""} data-source="${source.id}">
          <span class="source-order">${source.disabled ? "—" : index}</span>
          <span><strong>${source.kind}</strong><small>${source.label}</small></span>
          <span class="source-detail">${source.detail}</span>
        </button>`,
    )
    .join("");
}

function primaryActions() {
  return `
    <div class="actions">
      <button class="primary" type="button" data-action="watch">▶ Watch now</button>
      <button class="secondary" type="button" data-action="sources">Other sources</button>
      <button class="quiet" type="button" data-action="request">＋ Download</button>
    </div>`;
}

function catalogA() {
  return `
    <main class="variant-a catalog-screen">
      <header class="brand"><b>REEL</b><span>Prototype · one title</span></header>
      <section class="catalog-stage">
        <p class="eyebrow">Tonight's feature</p>
        <button class="movie-poster" type="button" data-action="open">${posterMarkup()}<span>Open movie →</span></button>
      </section>
    </main>`;
}

function detailA() {
  return `
    <main class="variant-a detail-screen">
      <button class="back" type="button" data-action="back">← Library</button>
      <div class="detail-backdrop"></div>
      <section class="hero-detail">
        <div class="detail-copy">
          <p class="eyebrow">${movie.genres.join(" · ")}</p>
          <h1>${movie.title}</h1>
          <p class="metadata">${movie.year} · ${movie.runtime} · ${movie.rating}</p>
          <p class="synopsis">${movie.synopsis}</p>
          ${primaryActions()}
          <p class="resolution-note">Jellyfin unavailable · Watch Now will try Stremio sources in add-on order.</p>
        </div>
        <div class="detail-poster">${posterMarkup()}</div>
      </section>
      ${state.sourcesVisible ? `<section class="source-drawer"><div><p class="eyebrow">Other sources</p><h2>Add-on order</h2></div><div class="source-list">${sourceRows()}</div></section>` : ""}
      ${playbackOverlay()}
    </main>`;
}

function catalogB() {
  return `
    <main class="variant-b catalog-screen">
      <header class="editorial-header"><b>Reel / 001</b><span>A catalogue of one</span><time>2026</time></header>
      <button class="editorial-movie" type="button" data-action="open">
        <span class="giant-index">01</span>
        <span class="editorial-title"><small>Open feature</small><strong>${movie.title}</strong><em>${movie.year} — ${movie.runtime}</em></span>
        <span class="editorial-art">${posterMarkup()}</span>
      </button>
    </main>`;
}

function detailB() {
  return `
    <main class="variant-b detail-screen">
      <header class="editorial-header"><button type="button" data-action="back">Reel / Library</button><span>Playback decision</span><time>${movie.year}</time></header>
      <section class="decision-grid">
        <div class="decision-copy">
          <p class="eyebrow">Feature 001</p><h1>${movie.title}</h1>
          <p class="synopsis">${movie.synopsis}</p>${primaryActions()}
        </div>
        <div class="decision-sources">
          <div class="decision-heading"><span>Playback options</span><small>Ordered by configured add-on</small></div>
          ${sourceRows()}
        </div>
      </section>
      <footer class="decision-footer"><span>Default</span><strong>First resolvable Stremio source</strong><span>No Reel ranking</span></footer>
      ${playbackOverlay()}
    </main>`;
}

function catalogC() {
  return `
    <main class="variant-c catalog-screen">
      <header class="compact-header"><b>reel</b><button>⌕</button><span class="avatar">E</span></header>
      <section class="shelf"><p>Continue exploring</p><button class="wide-tile" type="button" data-action="open">${posterMarkup()}<span><small>${movie.year} · ${movie.runtime}</small><strong>${movie.title}</strong><em>View details</em></span></button></section>
    </main>`;
}

function detailC() {
  return `
    <main class="variant-c detail-screen">
      <header class="compact-header"><button type="button" data-action="back">←</button><b>reel</b><span class="avatar">E</span></header>
      <section class="compact-detail">
        <div class="compact-art">${posterMarkup()}</div>
        <div><p class="eyebrow">${movie.genres.join(" · ")}</p><h1>${movie.title}</h1><p class="metadata">${movie.year} · ${movie.runtime} · ${movie.rating}</p><p class="synopsis">${movie.synopsis}</p>${primaryActions()}</div>
      </section>
      <section class="bottom-sheet">
        <div class="sheet-handle"></div><div class="decision-heading"><span>Available options</span><small>Tap any source to override</small></div>
        <div class="source-list horizontal">${sourceRows()}</div>
      </section>
      ${playbackOverlay()}
    </main>`;
}

function playbackOverlay() {
  if (state.activation === "idle") return "";
  const selected = sources.find((source) => source.id === state.selectedSource);
  return `
    <div class="playback-overlay">
      <div class="playback-card">
        <span class="pulse">▶</span><p class="eyebrow">Playback descriptor ready</p>
        <h2>${movie.title}</h2><p>${selected?.kind ?? "Stremio"} · ${selected?.label ?? "First available"}</p>
        <code>https://stream.local/play/${selected?.id ?? "s1"}</code>
        <button type="button" data-action="close-playback">Close prototype player</button>
      </div>
    </div>`;
}

function render() {
  const variant = currentVariant();
  variantLabel.textContent = `${variant.key} — ${variant.name}`;
  document.body.dataset.variant = variant.key;

  if (state.view === "catalog") {
    app.innerHTML = variant.key === "A" ? catalogA() : variant.key === "B" ? catalogB() : catalogC();
  } else {
    app.innerHTML = variant.key === "A" ? detailA() : variant.key === "B" ? detailB() : detailC();
  }

  stateElement.textContent = JSON.stringify(state, null, 2);
}

function activate(sourceId = "s1") {
  state.sourcesDiscovered = true;
  state.activation = "ready";
  state.selectedSource = sourceId;
  state.message = `Activated ${sourceId}; playback descriptor is ready.`;
  render();
}

document.addEventListener("click", (event) => {
  const target = event.target.closest("button");
  if (!target) return;

  if (target.dataset.variantDirection) {
    const current = variants.findIndex((variant) => variant.key === currentVariant().key);
    const next = (current + Number(target.dataset.variantDirection) + variants.length) % variants.length;
    const url = new URL(window.location);
    url.searchParams.set("variant", variants[next].key);
    history.replaceState({}, "", url);
    Object.assign(state, {
      sourcesDiscovered: state.view === "detail" && variants[next].key !== "A",
      sourcesVisible: state.view === "detail" && variants[next].key !== "A",
      activation: "idle",
      selectedSource: null,
      message: `Switched to variant ${variants[next].key}; transient playback state reset.`,
    });
    render();
    return;
  }

  if (target.dataset.source) {
    activate(target.dataset.source);
    return;
  }

  switch (target.dataset.action) {
    case "open":
      state.view = "detail";
      state.sourcesDiscovered = currentVariant().key !== "A";
      state.sourcesVisible = currentVariant().key !== "A";
      state.message = "Movie opened; no source has been activated.";
      break;
    case "back":
      Object.assign(state, { view: "catalog", sourcesDiscovered: false, sourcesVisible: false, activation: "idle", selectedSource: null, message: "Returned to the one-title catalogue." });
      break;
    case "sources":
      state.sourcesDiscovered = true;
      state.sourcesVisible = true;
      state.message = "Sources discovered without activation.";
      break;
    case "watch":
      activate("s1");
      return;
    case "request":
      state.message = "Download is a separate action; no playback source activated.";
      break;
    case "close-playback":
      state.activation = "idle";
      state.message = "Prototype player closed; selected source retained.";
      break;
    default:
      return;
  }
  render();
});

document.addEventListener("keydown", (event) => {
  if (!["ArrowLeft", "ArrowRight"].includes(event.key)) return;
  if (event.target.matches("input, textarea, [contenteditable]")) return;
  const direction = event.key === "ArrowLeft" ? -1 : 1;
  document.querySelector(`[data-variant-direction="${direction}"]`).click();
});

window.addEventListener("popstate", render);
render();
