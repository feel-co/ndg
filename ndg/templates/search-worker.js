self.onmessage = function(e) {
  const { type, data } = e.data;

  if (type === 'tokenize') {
    const tokens = data.toLowerCase()
      .match(/\b[a-zA-Z0-9_-]+\b/g) || []
      .filter(word => word.length > 2)
      .filter((word, index, arr) => arr.indexOf(word) === index);

    self.postMessage({ type: 'tokens', data: tokens });
  }

  if (type === 'search') {
    const { documents, query, limit } = data;
    const searchTerms = query.toLowerCase()
      .match(/\b[a-zA-Z0-9_-]+\b/g) || []
      .filter(word => word.length > 2);

    const docScores = new Map();

    searchTerms.forEach(term => {
      documents.forEach((doc, docId) => {
        if (doc.title.toLowerCase().includes(term) || doc.content.toLowerCase().includes(term)) {
          const score = doc.title.toLowerCase() === term ? 30 :
                       doc.title.toLowerCase().includes(term) ? 10 : 2;
          docScores.set(docId, (docScores.get(docId) || 0) + score);
        }
      });
    });

    const results = Array.from(docScores.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, limit)
      .map(([docId, score]) => ({ ...documents[docId], score }));

    self.postMessage({ type: 'results', data: results });
  }
};