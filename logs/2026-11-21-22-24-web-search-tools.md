# web search tools

## Context

Noah asked why Codex can look up stock prices and how Agent could do the same.
He also asked where OpenAI lists hosted tools and what Agent means by a host ID.
He then requested hosted web search and asked how domain filters reach the model.

## Decisions

- Enable hosted search without an app-defined allowlist or blocklist.
- Noah requested a single search function for ordinary and configured lookups. The model
  supplies a query and optional search settings. Hosted search now runs on the next main-loop
  request with full conversation history. This replaces the query-only research subrequest.

## Changes

- Added a search function with domain filters, context size, location, content types, and image
  settings. Agent returns search actions, citations, and requested image results to the model.
- Replaced the stock-price example with general instructions for current facts, verification,
  and explicit lookup requests.
- Added Markdown links for URL citations missing from the response text.
- Keep expanded source lists in the raw event journal. Strip them before saving hosted actions
  into model history, and trim legacy function results on replay without rewriting old records.

- Show all image results inside the collapsed search details in Apple clients. Load the thumbnail first, fall back
  to the original image, and link the captioned preview to the image and its source page.
- Recover image previews for existing chats from the stored search result.

- Pierre Words displays a returned HTTPS image inline when the assistant selects it with Markdown
  image syntax. Ordinary links keep their usual behavior.

- When hosted search returns images, let the model compose the reply in the same chat with
  the exact result URLs and captions. Keep the hosted draft in the event log.

## Verification

- Traced the computer tool's argument schema, action examples, and result delivery to explain
  how the model receives job details and screenshots. Compared this with hosted search.
- Confirmed Agent uses the public Responses API.
- Checked OpenAI's web search guide for the hosted tool and finance sources. Verified account access in the live check below.
- Checked the Responses API tool catalog. Confirmed that Agent requires a host argument for both
  device functions and checks access before dispatch.
- All 52 Rust tests passed with local socket access, including request and result mapping tests.
  Formatting, Clippy, and diff checks passed.
- Attempted a live AAPL lookup with the current search schema against the Responses API.
  OpenAI rejected the local API key with HTTP 401 `expired_secret_key` before the first
  tool call. The signed-in central service runs separately. The paired-service test below covers the actual login.
- The paired service's live check_model request succeeded using its ChatGPT login.
  gpt-6.2-sol completed the expected tool call and returned 1989.
- Submitted a live AAPL lookup through the signed-in paired service with no enabled devices.
  The conversation completed, but reported no search tool and returned no quote. An SSH
  query for the mini's launch configuration stopped at authentication and was canceled.
- The mini was running an older service. Installed a signed standalone service build through
  the paired device connection and verified its digest and Developer ID before startup.
- Corrected the hosted request to use a message array and explicitly select web search.
  Preserved streamed output items when the completion event omits them.
- In test chat `746e63a2-0d5c-46b4-b11c-990e1331cca5`, verified a search call, returned findings
  with URL citations, and the final answer with all device tools disabled. Agent reported AAPL at
  $344.87 after hours on Nov 20 at 7:59:58 p.m. EST and a $342.56 regular close.
- Verified the deployed change in “AAPL conversation context check.” The hosted answer used the
  ticker and formatting instruction from the previous turn. Search events contained source lists,
  saved hosted actions omitted them, and the next turn recalled the quote. All 55 tests passed.
- Tested allowlisting, blocking, and image search through the paired service with devices
  disabled. Verified each option in the hosted tool configuration returned by OpenAI.
  The allowlist yielded four NASA hosts. The blocklist yielded seven hosts outside NASA
  and Wikipedia. The image search requested up to two captioned results and returned one
  1024 × 627 image with a caption, thumbnail, and source page. Its URL returned HTTP 200
  with content type image/jpeg.
- Built and relaunched the signed Mac app and updated the paired service. The Golden Gate
  chat returned one image on its search row, identical to the original OpenAI event.
  All 55 tests and Clippy passed. The on-screen preview was not visually verified.
- Checked the saved Golden Gate reply. It used a Markdown link. All four Markdown tests passed,
  and a native WebKit check rendered one link and decoded one 1024-pixel image when given an
  explicit image embed. Rebuilt and relaunched the signed Mac app.
- The original banana chat returned one image but answered with a product link. An instruction-only
  retry embedded an unrelated URL. With the returned URLs provided to the model, the same prompt
  in “Banana picture verification” embedded a matching result. The Mac renderer decoded the
  selected image at 1200 pixels wide. All 55 Rust tests and Clippy passed.
