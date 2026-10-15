browser.runtime.onStartup.addListener(() => {
  console.log("Gemini Local Translator background started.");
});

browser.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === "translate") {
    fetch('http://127.0.0.1:8080/translate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ text: request.text })
    })
    .then(async response => {
      if (!response.ok) throw new Error(`Server error: ${response.status}`);
      return response.json();
    })
    .then(data => sendResponse({ success: true, translation: data.translation }))
    .catch(error => sendResponse({ success: false, error: error.message }));
    
    return true; 
  }
});
