# Product Design

## Product Intent

Happy Wakey is a daytime desktop command center. Its primary job is to help the user understand and tackle the day through a dependable agenda and calendar reminders. It also keeps weather, markets, important news, and frequently used pages in one stable window.

The product should reduce browser-tab sprawl and context switching without turning into another noisy feed. The ideal experience is quiet, fast, glanceable, and trustworthy enough to remain open all day.

## Audience

The initial audience is a desktop-heavy user who:

- starts work early;
- checks several calendars;
- monitors a small set of locations and market instruments;
- only wants news matching explicit interests;
- revisits a small set of operational pages;
- values portable, inspectable configuration;
- uses macOS, Windows, or Linux.

## Design Principles

### One Window, Clear Places

The sidebar is stable and always uses the same order:

1. Home
2. Calendar
3. Weather
4. Stocks
5. News
6. Devices
7. Settings

Home is the summary. Each other panel is the focused workspace. Users should never need to guess whether a command affects one panel or the whole app.

### High Signal, Low Ceremony

The interface favors compact headers, restrained cards, visible loading state, and plain commands. It avoids a marketing-style landing page inside the product.

### Honest State

The UI must distinguish:

- no configuration;
- not authenticated;
- loading;
- loaded but empty;
- partial success;
- provider failure;
- stale data.

An empty white area must never be the only signal that something went wrong.

### User-Controlled Relevance

Weather locations, stock symbols, news keywords, and browser bookmarks are explicitly selected by the user. News results are checked locally after the provider query so a provider cannot quietly broaden relevance beyond the configured keywords.

### Daytime Comfort

The app uses a light theme for heavy use from 5:00 AM through 3:00 PM. From 5:00 AM to 8:00 AM it changes to a softer, warmer palette with lower apparent brightness. After 8:00 AM it uses a clearer neutral light palette.

The time is re-evaluated every minute by `Theme.qml`; no restart is needed when the theme period changes.

## Information Architecture

### Home

Home is a six-card summary:

- Calendar: next events and authentication status.
- Weather: favorite count and current-condition previews.
- Stocks: watchlist count and quote previews.
- News: keyword count and matched-headline previews.
- Devices: Bluetooth support, connected device, and scan state.
- Setup: account, sync, API, and backup state.

Each data card has a local Refresh action and an Open action. Refresh All is disabled while any panel refresh is active, which prevents accidental duplicate sweeps.

### Calendar

The calendar is the product's organizing center. The current screen fetches the current Monday-through-Sunday window and lists events. The target design is a true seven-column weekly time grid with:

- all-day row;
- local-time conversion;
- current-time indicator;
- provider color;
- overlapping-event layout;
- click-through event detail;
- reminder controls.

The Home view should lead with a daily agenda rather than a generic event count. Before the workday it should show:

- the next event and time until it starts;
- number of scheduled meetings and total meeting time;
- travel/join preparation where known;
- conflicts and unusually dense blocks;
- invitations requiring a response;
- the user's chosen focus block;
- a concise "tackle the day" morning summary.

Reminder actions should include Open, Join, Snooze, and Dismiss. A reminder must identify which calendar/account produced the event and must not duplicate the same meeting when it appears through Google, Gmail, and Calendly.

### Weather

Each location card shows:

- current temperature and condition;
- feels-like temperature;
- humidity;
- wind speed;
- current precipitation;
- five forecast days with high, low, condition, and precipitation probability;
- provider and observation time;
- Radar command.

The layout uses two columns when enough width is available and one column at narrower desktop sizes. The data source is visibly attributed.

### Stocks

The watchlist is optimized for scanning symbol, current price, absolute change, and percentage change. The next design pass should add:

- asset type and exchange;
- quote timestamp and market state;
- compact intraday sparkline;
- user-defined ordering;
- currency-aware formatting;
- explicit delayed-data labeling.

The app is a monitoring interface, not a brokerage. Trading should not be added without a separate security, suitability, and transaction-confirmation design.

### News

The News view shows no more than five matching items. Each item includes title, short description, source, date, and optional image. Clicking opens the original publisher URL externally.

The target relevance model should support:

- must-match keywords;
- excluded keywords;
- source allow/deny lists;
- geographic scope;
- recency window;
- duplicate-story clustering.

### Devices

The Devices workspace uses the operating system's native BLE stack. It scans
only for peripherals advertising the Happy Wakey service UUID, exposes explicit
connect and disconnect controls, and sends a small versioned preview-alarm
command to the product characteristic. It never carries auth tokens, email,
owner IDs, or server credentials. External quick links remain editable in
Settings and open in the system browser after HTTP/HTTPS validation.

### Settings

Settings owns account state and editable collections. Collection limits are enforced in Rust after QML submits JSON:

- five weather locations;
- twenty stock symbols;
- twenty news keywords;
- fifty browser bookmarks.

## Onboarding

Onboarding has five steps:

1. Welcome and product intent.
2. Account/provider connection.
3. Git backup destination.
4. Starter weather, market, news, and shortcut choices.
5. Ready/open dashboard.

Progress is saved after navigation. Local state allows onboarding to work before login. After login, the state is mirrored to a dedicated Supabase table and merged by completion and timestamp.

The current Continue and Open Dashboard controls use a fixed 44-pixel hit area. They were tested in the live native app after earlier versions had unreliable click targets.

## Interaction Rules

- A data refresh must never run on the GUI thread.
- A second click while the same refresh is active is ignored and the button remains disabled.
- Partial provider success should retain successful data and report failed items.
- Destructive actions should be explicit and reversible where possible.
- External URLs are normalized and restricted to HTTP/HTTPS.
- Status text belongs in the persistent footer, not in transient invisible logs.
- Empty states should include the next useful action.

## Accessibility Direction

Qt exposes standard Controls such as buttons and fields to platform accessibility APIs. Custom QML hit targets require explicit accessible names/roles in a future pass. The sidebar delegates and onboarding custom buttons should be upgraded so keyboard navigation and assistive technology do not depend on coordinate-based interaction.

The minimum supported window is currently 900 by 600 logical pixels. Every view must be verified at that size and at the default 1280 by 860 size without overlap or horizontal scrolling.
