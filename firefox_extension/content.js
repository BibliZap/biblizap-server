// Fonction pour créer un élément de lien avec une icône
function createLinkWithIcon(doi) {
  const link = document.createElement('a');
  link.href = `https://biblizap.org/biblizap-results?ids=${encodeURIComponent(doi)}`;
  link.target = '_blank';
  link.rel = 'noopener noreferrer';
  link.title = 'Search citations and references with BibliZap';
  link.setAttribute('aria-label', `${link.title}: ${doi}`);
  link.innerHTML = '<img src="data:image/png;base64,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" style="margin-left: 5px; width: 20px; height: 20px; cursor: pointer;" />'
  link.firstElementChild.alt = '';
  return link;
}

function highlightDOI() {
  const regex = /10\.\d{4,9}\/[-._;()\/:A-Za-z0-9]+(?:[-_;/():A-Za-z0-9]|(?<=\d)\.(?=\d))/gi;
  const textNodes = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);

  let nodesToReplace = [];

  for (let node = textNodes.nextNode(); node != null; node = textNodes.nextNode()) {
    let matches = node.nodeValue.match(regex);
    if (matches) {
      let lastIndex = 0;
      let span = document.createElement('span');

      node.nodeValue.replace(regex, (match, offset) => {
        // Ajouter le texte avant le DOI
        span.appendChild(document.createTextNode(node.nodeValue.slice(lastIndex, offset)));
        // Créer un span pour le DOI
        let doiSpan = document.createElement('span');
        doiSpan.appendChild(document.createTextNode(match));
        doiSpan.appendChild(createLinkWithIcon(match));
        span.appendChild(doiSpan);

        lastIndex = offset + match.length;
        return match;
      });

      // Ajouter le reste du texte après le dernier DOI
      span.appendChild(document.createTextNode(node.nodeValue.slice(lastIndex)));
      nodesToReplace.push({oldNode: node, newNode: span});
    }
  }

  // Remplacer les anciens nœuds par les nouveaux
  nodesToReplace.forEach(({oldNode, newNode}) => {
    oldNode.parentNode.replaceChild(newNode, oldNode);
  });
}



// Appliquer la fonction à la page
highlightDOI();
