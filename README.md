This is a **Local Gemini Web Translator Extension & Server** which is a privacy-focused, self-hosted system that allows you to translate entire web pages in Firefox using **Google Gemini’s web interface** as the translation engine—bypassing traditional paid translation APIs while keeping your data local.

![Demo](https://github.com/user-attachments/assets/9c853a29-f40a-4e82-a8d2-cae1ad37cdb9 "Translating 'https://shamela.ws/book/6388/1'")
---

### How the Project Works

The system is split into two main parts that talk to each other over localhost:

#### 1. The Local Rust Backend (`main.rs` & `auth.rs`)

* **Automated Authentication:** Instead of requiring you to manually copy API keys or login tokens, it looks directly at your local Firefox profile (specifically targeting a "hardened" profile), copies `cookies.sqlite` to `/tmp` to bypass file locks, and uses `sqlite3` to query your active Google session cookies (`__Secure-1PSID` and `__Secure-1PSIDTS`).
* **Session Token Extraction:** It connects to `gemini.google.com` using those cookies to dynamically scrape live security tokens (`at` / `SNlM0e`) and build versions (`bl`) required by Google's internal web frontend.
* **Axum API Server:** It runs a lightweight HTTP server on `[http://127.0.0.1:8080](http://127.0.0.1:8080)` with CORS enabled. When it receives a translation request, it packages the prompt, hits Google's internal `StreamGenerate` endpoint, parses the complex chunked JSON response, and returns clean text.

#### 2. The Firefox Web Extension (`popup.html`, `popup.js`, `manifest.json`)

* **DOM Text Extraction:** When you click "Translate Current Page," a script runs in your active tab using a `TreeWalker` to gather all meaningful text nodes while safely ignoring code blocks, scripts, and navigation elements.
* **Smart Batching:** It groups the extracted text into safe character batches (around 3,500 characters) separated by `===SPLIT===` markers to avoid hitting request size limits.
* **Direct Communication:** It sends these batches via `fetch()` to your local Rust server.
* **Seamless Re-injection:** Once the translated text comes back, it splits the response apart and injects the English text back into the exact same DOM text nodes. This preserves all original fonts, colors, layouts, and inline HTML styling without breaking the page.

---

## 🛠️ Getting Started

Here is everything you need to install and how to run your project step by step.

---

### 📦 Prerequisites (What to Install)

Before running the project, make sure you have the following installed on your system (assuming Linux/Ubuntu/Fedora/Arch based on the path setups):

1. **Rust & Cargo** (for the backend server):
If you don't have Rust installed, run this in your terminal:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

```


*(Restart your terminal or run `source "$HOME/.cargo/env"`)*

2. **SQLite3 Command-Line Tool** (required by `auth.rs` to read your Firefox cookies):
* Ubuntu/Debian: 
```bash
sudo apt install sqlite3
```
* Fedora:
```bash 
sudo dnf install sqlite
```
* Arch Linux: 
```bash 
sudo pacman -S sqlite
```


3. **Firefox Browser** (to host the extension and store your Gemini session).
* Make sure you are logged into **[gemini.google.com](https://gemini.google.com)** in your Firefox browser (specifically using your `"hardened"` profile as configured in `auth.rs`).



---

### 🚀 How to Run the Project

You will run the project in two parts: starting your local Rust backend, and loading the extension into Firefox.

#### Step 1: Clone this repository to your local machine:

```bash
git clone https://github.com/mohsince-04/rust-gemini-translator.git
```
Ensure all your files are inside a single project folder `rust-gemini-translator/` structured like this:

```
📂 gemini-local-translator/
├── Cargo.toml
├── README.md
├── manifest.json
├── popup.html
└── popup.js
└── 📂 src/
    ├── main.rs
    └── auth.rs
```

#### Step 2: Start the Rust Backend Server

Before starting it,

1. **Locate Your Profile Path:** Firefox.

1. Open Firefox and type `about:profiles` in the address bar, then press **Enter**.
2. Look for the profile currently in use (usually labeled **default** or **default-release**).
3. Find the **Root Directory** path listed there and copy it to your clipboard.

* **Verification:** The path should look similar to `/home/username/.mozilla/firefox/xxxxxxxx.default-release`.


2. **Update auth.rs:** Terminal / Editor.

1. Open your `auth.rs` file in your editor.
2. Locate the line where the path is defined and paste your copied root directory path.
3. Append `/cookies.sqlite` to the end of the path string.

* **Verification:** Check that the path string ends with `/cookies.sqlite` inside your code.


---

After this,

1. Open your terminal inside the project folder.
2. Run the server in release mode:
```bash
cd rust-gemini-translator && cargo run --release

```


3. **What happens next:**
* It will automatically grab your session cookies from your Firefox profile.
* It will fetch the dynamic tokens from Gemini.
* You should see a green success message in your terminal:
`[+] Translation Server running at http://127.0.0.1:8080/translate`



#### Step 3: Load the Extension into Firefox

1. Open Firefox.
2. Type `about:debugging` in the address bar and press Enter.
3. Click on **This Firefox** on the left-hand menu.
4. Click the **Load Temporary Add-on...** button.
5. Navigate to your project folder and select your `manifest.json` file.

#### Step 4: Test It Out!

1. Go to any foreign-language webpage in Firefox.
2. Click the extension icon in your browser toolbar.
3. Click **Translate Current Page**.
4. Watch your terminal log the incoming request, and watch the page text seamlessly translate into English while preserving all original formatting!
