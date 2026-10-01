# BibliZap browser extension

Shared source for Firefox and Chrome. The Firefox add-on is available on
[Mozilla Add-ons](https://addons.mozilla.org/addon/biblizap/).

The extension adds a BibliZap icon beside DOIs on web pages. Clicking the icon
opens a new tab and immediately searches BibliZap using that DOI as the seed.
The results page uses the standard defaults: depth 2, both citations and
references, and up to 100 results. The seed and search settings remain editable
on the results page. Detecting a DOI alone does not start a search.

Links use `https://biblizap.org/biblizap-results?ids=<encoded DOI>`.

## Packaging

Requires Bash and `zip`. From the repository root, run:

```bash
./extension/package.sh
```

This builds both browsers. Use `./extension/package.sh firefox` or
`./extension/package.sh chrome` to build just one.

The script copies the shared `content.js` and `BBZ.png` and the selected manifest
into an unpacked directory, renaming the manifest to `manifest.json`. It also
creates a ZIP with those three files at its root:

```text
extension/dist/
  firefox/
  chrome/
  biblizap-firefox.zip
  biblizap-chrome.zip
```

Only edit the shared source and `manifests/firefox.json` or
`manifests/chrome.json`; `dist/` is generated and ignored by Git. Keep the name,
version, description and content-script configuration aligned in both manifests.
The current script uses an embedded image and needs no web-accessible resources
or background script. The Chrome package uses Manifest V3; the Firefox package
keeps Manifest V2.

## Local testing

For Firefox, open `about:debugging#/runtime/this-firefox`, click
**Load Temporary Add-on**, and select `extension/dist/firefox/manifest.json`.

For Chrome, open `chrome://extensions`, enable **Developer mode**, click
**Load unpacked**, and select `extension/dist/chrome/`.

After editing the source, rerun the packaging script, reload the extension in
the browser's extension manager, and refresh the web page where you want to
detect DOIs. Packaging does not publish either extension.
