# Sakuya Assistant - Pitch Notes

## The Three Reasons Ideas Fail

Ideas fail for three reasons: **lack of market**, **poor strategy**, and **missing assets**. Sakuya addresses all three — both for itself as a product, and as a tool that helps *other* businesses avoid these failures.

### How Sakuya avoids each failure mode

| Failure reason | How Sakuya avoids it | How Sakuya helps *your* business avoid it |
|---|---|---|
| **Lack of market** | Every business needs data tools but can't afford consultants or enterprise software. SMBs are massively underserved. | Competitor analysis reveals where the uncontested market space is — Blue Ocean Strategy finds the gaps others missed. |
| **Poor strategy** | LLM-agnostic = no vendor lock-in. Desktop-first = works with sensitive data. One-click install = adoption isn't a barrier. | Blue Ocean's Four Actions Framework (Eliminate/Reduce/Raise/Create) gives you a concrete strategic playbook, not just data. |
| **Missing assets** | Built on proven open-source infra (OpenClaw, Electron, FastAPI). The AI engine handles what you'd otherwise need a full dev team for. | Connects to assets you already have — Salesforce, Google Sheets, databases, APIs. Turns your existing data into strategic insight. |

### The adoption angle

The judges identified **adoption** as the key barrier: great tools don't create impact if businesses can't or won't use them. Sakuya is built specifically to solve this.

1. **Too technical** — most AI tools require CLI, API keys, config files, Docker, Python environments. Small business owners don't have a sysadmin.
2. **Vendor lock-in anxiety** — committing to one AI provider feels risky. What if prices change? What if a better model comes out?
3. **"Cool demo, now what?"** — chatbots are impressive in demos but businesses need them to actually *do things* — organize files, check spreadsheets, analyze competitors, query databases.
4. **Integration gap** — businesses already use Salesforce, Google Sheets, databases. A tool that doesn't connect to those is a toy.

| Adoption barrier | Sakuya's answer |
|---|---|
| Too technical | **One-click installer.** Double-click an `.exe`, it sets up everything — WSL, OpenClaw, browser automation. Zero command line. |
| Vendor lock-in | **LLM-agnostic.** Powered by OpenClaw, which works with *any* LLM — OpenAI, Anthropic, local models, whatever you already pay for. Switch anytime. |
| "Cool demo, now what?" | **9 real business skills.** File organization, Excel error checking, data processing, competitor analysis, pipeline building — not just chat. |
| Integration gap | **Connects to your stack.** Salesforce, Google Sheets, PostgreSQL/MySQL/SQLite/MongoDB, any REST API via OpenAPI discovery. |

## The 60-Second Pitch

> "Every team here built something cool. But the judges asked the right question: **will businesses actually use it?**
>
> We built Sakuya Assistant — a desktop AI that doesn't just chat, it *works*. Give it your competitors' URLs, it delivers a full Blue Ocean Strategy analysis. Point it at a messy spreadsheet, it finds the formula errors. Connect it to Salesforce, it queries your data.
>
> But here's what matters for adoption: **it's one click to install, and it works with any AI model.** No CLI. No Docker. No API keys to manage. Your non-technical office manager can run it.
>
> And because it's built on OpenClaw — an LLM-agnostic engine — you're never locked into one provider. Use GPT, Claude, Llama, whatever. The AI can even browse the web for you through browser automation.
>
> This isn't a prototype. It's 35 API endpoints, 15 test files, a working installer, and a competitor analysis engine that replaces a strategy consultant. **Adoption isn't our problem — we designed it away.**"

## Key Stats to Drop

- **35 REST endpoints** + streaming WebSocket chat
- **9 business skills** (file org, Excel checking, data processing, Salesforce, Google Suite, databases, API discovery, competitor analysis, pipeline builder)
- **15 test files** (9 backend + 6 frontend) with CI/CD
- **One `.exe` installer** — bootstraps WSL + OpenClaw + browser relay
- **Blue Ocean Strategy** — framework used by Fortune 500, automated for SMBs
- **LLM-agnostic** — works with OpenAI, Anthropic, local models, anything

## Handling Q&A

### "How is this different from ChatGPT?"
ChatGPT is a chatbot. Sakuya is a business tool that *happens to chat*. It organizes your files, checks your spreadsheets, queries your Salesforce, analyzes your competitors. ChatGPT can't connect to your database or scrape competitor websites with browser automation.

### "Why not just build a web app?"
Desktop app = runs locally, accesses local files directly, works offline (with local models), no cloud hosting costs. For a business data tool that handles sensitive files and databases, local-first is the right architecture.

### "What about security / data privacy?"
Everything runs locally. Your data never leaves your machine. The LLM can be local too (Llama, etc.). The only external calls are to whatever LLM provider *you* choose, and even that's optional.

### "How does the competitor analysis work?"
Web scraping with browser automation (OpenClaw Browser Relay). Give it competitor URLs → it scrapes their sites → extracts competitive factors (pricing, features, channels) → builds a Blue Ocean Strategy Canvas → applies the Four Actions Framework (Eliminate, Reduce, Raise, Create) → surfaces uncontested market opportunities. Same framework McKinsey charges $50k+ for.

### "What's the business model?"
Freemium desktop app. Free tier with basic skills, paid tier unlocks competitor analysis, advanced integrations, and priority support. The competitor analysis alone justifies a $50-100/month subscription — businesses currently pay consultants thousands for equivalent output.

### "How do you handle the adoption problem you're describing?"
That's the whole point. Three things: (1) one-click installer that handles all dependencies, (2) LLM-agnostic so no vendor lock-in fear, (3) connects to tools businesses already use (Salesforce, Google Sheets, databases). We didn't build a cool AI demo — we built something an office manager can install and use on day one.

## Demo Flow (if applicable)

1. **Show the installer** — "One `.exe`, that's it"
2. **Open the app** — themed desktop UI, not a browser tab
3. **Send a chat message** — show streaming response from OpenClaw
4. **Run competitor analysis** — give it 2-3 competitor URLs, show the Blue Ocean Strategy Canvas output
5. **Check a spreadsheet** — drop in a messy Excel file, show error detection
6. **Show the Salesforce/database connection** — "it connects to your existing stack"
7. **Close with adoption** — "Everything you just saw, a non-technical user can set up in one click"
