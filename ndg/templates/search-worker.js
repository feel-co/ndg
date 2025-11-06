self.onmessage = function(e) {
  const { type, data } = e.data;

  if (type === 'tokenize') {
    const tokens = (typeof data === 'string' ? data : '')
      .toLowerCase()
      .match(/\b[a-zA-Z0-9_-]+\b/g) || []
      .filter(word => word.length > 2);
    
    const uniqueTokens = Array.from(new Set(tokens));

    self.postMessage({ type: 'tokens', data: uniqueTokens });
  }

  if (type === 'search') {
    const { documents, query, limit } = data;
    const searchTerms = query.toLowerCase()
      .match(/\b[a-zA-Z0-9_-]+\b/g) || []
      .filter(word => word.length > 2);

    const docScores = new Map();
    
    // Pre-compute lower-case terms once
    const lowerSearchTerms = searchTerms.map(term => term.toLowerCase());

    // Pre-compute lower-case strings for each document
    const processedDocs = documents.map((doc, docId) => ({
      docId,
      title: doc.title,
      content: doc.content,
      lowerTitle: doc.title.toLowerCase(),
      lowerContent: doc.content.toLowerCase()
    }));

    lowerSearchTerms.forEach(lowerTerm => {
      processedDocs.forEach(({ docId, title, content, lowerTitle, lowerContent }) => {
        if (lowerTitle.includes(lowerTerm) || lowerContent.includes(lowerTerm)) {
          const score = lowerTitle === lowerTerm ? 30 :
                       lowerTitle.includes(lowerTerm) ? 10 : 2;
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