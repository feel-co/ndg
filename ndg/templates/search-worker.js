const fuzzyMatch = (query, target) => {
  const lowerQuery = query.toLowerCase();
  const lowerTarget = target.toLowerCase();

  let queryIdx = 0;
  let targetIdx = 0;
  let score = 0;
  const matches = [];

  while (queryIdx < lowerQuery.length && targetIdx < lowerTarget.length) {
    if (lowerQuery[queryIdx] === lowerTarget[targetIdx]) {
      matches.push(targetIdx);
      score += 10;

      if (queryIdx > 0 && targetIdx > 0) {
        const prevMatchIdx = matches[matches.length - 2];
        const distance = targetIdx - prevMatchIdx - 1;

        if (distance === 1) {
          score += 15;
        } else if (distance === 2) {
          score += 5;
        } else if (distance === 3) {
          score += 2;
        }
      }

      queryIdx++;
      targetIdx++;
    } else {
      targetIdx++;
    }
  }

  if (queryIdx !== lowerQuery.length) {
    return null;
  }

  const lengthRatio = lowerQuery.length / lowerTarget.length;
  score *= lengthRatio;

  if (lowerTarget === lowerQuery) {
    score += 100;
  } else if (lowerTarget.startsWith(lowerQuery)) {
    score += 50;
  } else if (lowerTarget.includes(lowerQuery)) {
    score += 30;
  }

  const maxScore = lowerQuery.length * 15 + 100;
  const normalizedScore = score / maxScore;

  return normalizedScore >= 0.3 ? normalizedScore : null;
};

const levenshteinDistance = (str1, str2) => {
  const m = str1.length;
  const n = str2.length;

  if (m === 0) return n;
  if (n === 0) return m;
  if (Math.abs(m - n) > 3) return 999;

  const dp = Array(m + 1).fill(null).map(() => Array(n + 1).fill(0));

  for (let i = 0; i <= m; i++) dp[i][0] = i;
  for (let j = 0; j <= n; j++) dp[0][j] = j;

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (str1[i - 1] === str2[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1];
      } else {
        dp[i][j] = 1 + Math.min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1]);
      }
    }
  }

  return dp[m][n];
};

self.onmessage = function(e) {
  const { messageId, type, data } = e.data;

  const respond = (type, data) => {
    self.postMessage({ messageId, type, data });
  };

  const respondError = (error) => {
    self.postMessage({ messageId, type: 'error', error: error.message || String(error) });
  };

  try {
    if (type === 'tokenize') {
      const tokens = (typeof data === 'string' ? data : '')
        .toLowerCase()
        .match(/\b[a-zA-Z0-9_-]+\b/g) || []
        .filter(word => word.length > 2);

      const uniqueTokens = Array.from(new Set(tokens));
      respond('tokens', uniqueTokens);
    }

    if (type === 'search') {
      const { documents, query, limit } = data;
      const rawQuery = query.toLowerCase();
      const searchTerms = (typeof query === 'string' ? query : '')
        .toLowerCase()
        .match(/\b[a-zA-Z0-9_-]+\b/g) || []
        .filter(word => word.length > 2);

      const useFuzzySearch = rawQuery.length >= 3;
      const pageMatches = new Map();

      // Pre-compute lower-case strings for each document
      const processedDocs = documents.map((doc, docId) => ({
        docId,
        doc,
        lowerTitle: doc.title.toLowerCase(),
        lowerContent: doc.content.toLowerCase()
      }));

      // First pass: Score pages with fuzzy matching
      processedDocs.forEach(({ docId, doc, lowerTitle, lowerContent }) => {
        let match = pageMatches.get(docId);
        if (!match) {
          match = { doc, pageScore: 0, matchingAnchors: [] };
          pageMatches.set(docId, match);
        }

        if (useFuzzySearch) {
          const fuzzyTitleScore = fuzzyMatch(rawQuery, lowerTitle);
          const fuzzyContentScore = fuzzyMatch(rawQuery, lowerContent);

          if (fuzzyTitleScore !== null) {
            match.pageScore += fuzzyTitleScore * 100;
          } else {
            const editDist = levenshteinDistance(rawQuery, lowerTitle);
            if (editDist <= Math.max(1, Math.floor(rawQuery.length * 0.3)) && editDist < rawQuery.length) {
              const typoScore = (1 - editDist / rawQuery.length) * 50;
              match.pageScore += typoScore;
            }
          }

          if (fuzzyContentScore !== null) {
            match.pageScore += fuzzyContentScore * 30;
          }
        }

        // Token-based exact matching
        searchTerms.forEach(term => {
          if (lowerTitle.includes(term)) {
            match.pageScore += lowerTitle === term ? 20 : 10;
          }
          if (lowerContent.includes(term)) {
            match.pageScore += 2;
          }
        });
      });

      // Second pass: Find matching anchors
      pageMatches.forEach((match) => {
        const doc = match.doc;
        if (!doc.anchors || doc.anchors.length === 0) return;

        doc.anchors.forEach(anchor => {
          const anchorText = anchor.text.toLowerCase();
          let anchorMatches = false;

          if (useFuzzySearch) {
            const fuzzyScore = fuzzyMatch(rawQuery, anchorText);
            if (fuzzyScore !== null && fuzzyScore >= 0.4) {
              anchorMatches = true;
            }
          }

          if (!anchorMatches) {
            searchTerms.forEach(term => {
              if (anchorText.includes(term)) {
                anchorMatches = true;
              }
            });
          }

          if (anchorMatches) {
            match.matchingAnchors.push(anchor);
          }
        });
      });

      const results = Array.from(pageMatches.values())
        .filter(m => m.pageScore > 5)
        .sort((a, b) => b.pageScore - a.pageScore)
        .slice(0, limit);

      respond('results', results);
    }
  } catch (error) {
    respondError(error);
  }
};