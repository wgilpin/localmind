import { c as create_ssr_component, a as subscribe, e as escape, b as add_attribute, v as validate_component } from "../../chunks/ssr.js";
import { w as writable } from "../../chunks/index.js";
const searchResults = writable("");
const showResultsSection = writable(false);
const showNewNoteSection = writable(false);
const Search = create_ssr_component(($$result, $$props, $$bindings, slots) => {
  return `<div class="flex space-x-2" data-svelte-h="svelte-dme4n5"><input type="text" id="search-input" placeholder="Enter search query" class="input"> <button id="search-button" class="btn">Search</button></div>`;
});
const Results = create_ssr_component(($$result, $$props, $$bindings, slots) => {
  let $showResultsSection, $$unsubscribe_showResultsSection;
  let $searchResults, $$unsubscribe_searchResults;
  $$unsubscribe_showResultsSection = subscribe(showResultsSection, (value) => $showResultsSection = value);
  $$unsubscribe_searchResults = subscribe(searchResults, (value) => $searchResults = value);
  $$unsubscribe_showResultsSection();
  $$unsubscribe_searchResults();
  return `${$showResultsSection ? `<div id="results-container">${$searchResults ? `<div class="search-result">${escape($searchResults)}</div>` : `<div class="no-results" data-svelte-h="svelte-z0kuyy">No results found.</div>`}</div>` : ``}`;
});
const NewNote = create_ssr_component(($$result, $$props, $$bindings, slots) => {
  let noteTitle = "";
  return `<div class="space-y-2"><input type="text" id="note-title" placeholder="Note Title" class="input"${add_attribute("value", noteTitle)}> <textarea id="note-content" placeholder="Note Content" class="textarea">${escape("")}</textarea> <button id="save-note-button" class="btn" data-svelte-h="svelte-415fk0">Save Note</button></div>`;
});
const FAB = create_ssr_component(($$result, $$props, $$bindings, slots) => {
  return `<button id="fab" class="btn" data-svelte-h="svelte-1ffa251">+</button>`;
});
const Page = create_ssr_component(($$result, $$props, $$bindings, slots) => {
  let $showResultsSection, $$unsubscribe_showResultsSection;
  let $showNewNoteSection, $$unsubscribe_showNewNoteSection;
  $$unsubscribe_showResultsSection = subscribe(showResultsSection, (value) => $showResultsSection = value);
  $$unsubscribe_showNewNoteSection = subscribe(showNewNoteSection, (value) => $showNewNoteSection = value);
  $$unsubscribe_showResultsSection();
  $$unsubscribe_showNewNoteSection();
  return `<div id="container"><h1 data-svelte-h="svelte-rf0gcz">LocalMind</h1> <section><h2 data-svelte-h="svelte-mqqb52">Search</h2> ${validate_component(Search, "Search").$$render($$result, {}, {}, {})}</section> ${$showResultsSection ? `<section id="results-section"><h2 data-svelte-h="svelte-s6ksa4">Results</h2> ${validate_component(Results, "Results").$$render($$result, {}, {}, {})}</section>` : ``} ${$showNewNoteSection ? `<section id="new-note-section"><h2 data-svelte-h="svelte-z923co">New Note</h2> ${validate_component(NewNote, "NewNote").$$render($$result, {}, {}, {})}</section>` : ``}</div> ${validate_component(FAB, "FAB").$$render($$result, {}, {}, {})}`;
});
export {
  Page as default
};
