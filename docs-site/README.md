# saywhen docs (generated)

This folder is generated from the Markdown in the repository. Do not edit the HTML by hand.

```bash
npm run docs:build
```

The build uses Python 3 and `mistune`. It creates:

- `index.html`, one page per Markdown document, `docs.css` and `docs.js`
- `search.json`, the client-side search index

Serve the folder with any static file server. The search needs HTTP; opening `index.html` from the file system will not load `search.json`.
