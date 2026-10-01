# BibliZap-Firefox
Source code for the Firefox add-on [BibliZap](https://addons.mozilla.org/addon/biblizap/)

The extension adds a BibliZap icon beside DOIs on web pages. Clicking the icon
opens a new tab and immediately searches BibliZap using that DOI as the seed.
The results page uses the standard defaults: depth 2, both citations and
references, and up to 100 results. The seed and search settings remain editable
on the results page. Detecting a DOI alone does not start a search.

Links use `https://biblizap.org/biblizap-results?ids=<encoded DOI>`.

To try the extension locally, open `about:debugging#/runtime/this-firefox`, click
**Load Temporary Add-on**, and select `manifest.json` from this directory.
After changing the extension, click **Reload** there and reload the page on
which you want to detect DOIs.
