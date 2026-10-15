document.getElementById('translateBtn').addEventListener('click', async () => {
  const btn = document.getElementById('translateBtn');
  const resultDiv = document.getElementById('result');

  btn.disabled = true;
  btn.textContent = 'Translating...';
  resultDiv.textContent = 'Extracting text nodes...';

  try {
    const tabs = await browser.tabs.query({ active: true, currentWindow: true });
    if (!tabs.length) throw new Error("No active tab found.");

    // 1. Extract meaningful text nodes directly using TreeWalker
    const extraction = await browser.scripting.executeScript({
      target: { tabId: tabs[0].id },
      func: () => {
        const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
          acceptNode: function(node) {
            const parent = node.parentNode;
            if (!parent) return NodeFilter.FILTER_REJECT;
            if (['SCRIPT', 'STYLE', 'NAV', 'HEADER', 'FOOTER', 'NOSCRIPT'].includes(parent.tagName)) {
              return NodeFilter.FILTER_REJECT;
            }
            return node.nodeValue.trim().length > 2 ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT;
          }
        });

        let node;
        const texts = [];
        while (node = walker.nextNode()) {
          texts.push(node.nodeValue.trim());
        }
        return texts;
      }
    });

    const texts = extraction[0]?.result || [];
    if (!texts.length) throw new Error("No translatable text found.");

    resultDiv.textContent = `Grouping ${texts.length} elements into batches...`;

    // 2. Group texts into batches of ~3500 characters
    const batches = [];
    let currentBatch = [];
    let currentLength = 0;

    for (const text of texts) {
      if (currentLength + text.length > 3500 && currentBatch.length > 0) {
        batches.push(currentBatch);
        currentBatch = [text];
        currentLength = text.length;
      } else {
        currentBatch.push(text);
        currentLength += text.length + 2;
      }
    }
    if (currentBatch.length > 0) {
      batches.push(currentBatch);
    }

    // 3. Translate each batch directly via local Rust server
    let allTranslatedTexts = [];
    for (let i = 0; i < batches.length; i++) {
      resultDiv.textContent = `Translating batch ${i + 1} of ${batches.length}...`;
      
      const combinedText = batches[i].join('\n\n===SPLIT===\n\n');

      const res = await fetch('http://127.0.0.1:8080/translate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ text: combinedText })
      });

      if (!res.ok) {
        throw new Error(`Server returned status ${res.status}`);
      }

      const data = await res.json();
      const translatedBatchString = data.translation || "";
      
      const splitTranslations = translatedBatchString.split(/\n*\s*===SPLIT===\s*\n*/);
      allTranslatedTexts.push(...splitTranslations);
    }

    resultDiv.textContent = 'Applying translations...';

    // 4. Inject translated text back into the exact same text nodes
    await browser.scripting.executeScript({
      target: { tabId: tabs[0].id },
      func: (newTranslations) => {
        const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
          acceptNode: function(node) {
            const parent = node.parentNode;
            if (!parent) return NodeFilter.FILTER_REJECT;
            if (['SCRIPT', 'STYLE', 'NAV', 'HEADER', 'FOOTER', 'NOSCRIPT'].includes(parent.tagName)) {
              return NodeFilter.FILTER_REJECT;
            }
            return node.nodeValue.trim().length > 2 ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT;
          }
        });

        let node;
        let index = 0;
        while ((node = walker.nextNode()) && index < newTranslations.length) {
          if (newTranslations[index]) {
            node.nodeValue = newTranslations[index];
          }
          index++;
        }
      },
      args: [allTranslatedTexts]
    });

    resultDiv.textContent = 'Translated successfully with original styling!';

  } catch (err) {
    console.error("Extension error:", err);
    resultDiv.textContent = `Error: ${err.message}`;
  } finally {
    btn.disabled = false;
    btn.textContent = 'Translate Current Page';
  }
});
