document.addEventListener("DOMContentLoaded", () => {
    if (isAuthenticated()) {
        showPage("dashboard");
        loadLinks();
    } else {
        showPage("login");
    }
    checkHealthStatus();
    updateUserSection();
});

function showPage(page) {
    document.querySelectorAll(".page").forEach((p) => (p.style.display = "none"));
    const target = document.getElementById(`page-${page}`);
    if (target) target.style.display = "block";
    updateUserSection();
}

function updateUserSection() {
    const authSection = document.getElementById("auth-section");
    const userSection = document.getElementById("user-section");

    if (isAuthenticated()) {
        authSection.style.display = "none";
        userSection.style.display = "flex";
        document.getElementById("user-email").textContent = getCurrentUser();
    } else {
        authSection.style.display = "flex";
        userSection.style.display = "none";
    }
}

async function handleShorten(event) {
    event.preventDefault();
    const url = document.getElementById("url-input").value;
    const errorEl = document.getElementById("shorten-error");
    errorEl.textContent = "";

    try {
        const data = await shortenUrl(url);
        const resultEl = document.getElementById("shorten-result");
        const linkEl = document.getElementById("short-url-link");
        linkEl.href = data.short_url;
        linkEl.textContent = data.short_url;
        resultEl.style.display = "block";
        document.getElementById("url-input").value = "";
        loadLinks();
    } catch (err) {
        errorEl.textContent = err.message;
    }
}

async function loadLinks() {
    const listEl = document.getElementById("links-list");
    const emptyEl = document.getElementById("links-empty");

    try {
        const data = await getLinks();
        listEl.innerHTML = "";

        if (data.links.length === 0) {
            emptyEl.style.display = "block";
            return;
        }

        emptyEl.style.display = "none";

        data.links.forEach((link) => {
            const item = document.createElement("div");
            item.className = "link-item";
            item.innerHTML = `
                <div class="link-info">
                    <a href="${link.short_url}" target="_blank" class="short-code">${link.short_code}</a>
                    <div class="original-url" title="${link.original_url}">${link.original_url}</div>
                </div>
                <div class="link-meta">
                    <span class="clicks">${link.clicks} clicks</span>
                    <span>${new Date(link.created_at).toLocaleDateString()}</span>
                    <button class="stats-btn" onclick="showStats('${link.short_code}')">Stats</button>
                    <button class="delete-btn" onclick="handleDelete('${link.short_code}')">Delete</button>
                </div>
            `;
            listEl.appendChild(item);
        });
    } catch (err) {
        listEl.innerHTML = `<p class="error">Failed to load links: ${err.message}</p>`;
    }
}

async function showStats(shortCode) {
    const section = document.getElementById("link-stats-section");
    section.style.display = "block";

    try {
        const data = await getLinkStats(shortCode);
        document.getElementById("stat-total-clicks").textContent = data.total_clicks;
        document.getElementById("stat-unique-visitors").textContent = data.unique_visitors;

        const countriesEl = document.getElementById("stats-countries");
        if (data.clicks_by_country && Object.keys(data.clicks_by_country).length > 0) {
            countriesEl.innerHTML = Object.entries(data.clicks_by_country)
                .map(([country, count]) => `<div class="bar-item"><span class="bar-label">${country}</span><div class="bar-fill" style="width: ${Math.min(count / Math.max(...Object.values(data.clicks_by_country)) * 100, 100)}%">${count}</div></div>`)
                .join("");
        } else {
            countriesEl.innerHTML = "<p>No country data yet</p>";
        }

        const browsersEl = document.getElementById("stats-browsers");
        if (data.browsers && Object.keys(data.browsers).length > 0) {
            browsersEl.innerHTML = Object.entries(data.browsers)
                .map(([name, count]) => `<div class="bar-item"><span class="bar-label">${name}</span><div class="bar-fill" style="width: ${Math.min(count / Math.max(...Object.values(data.browsers)) * 100, 100)}%">${count}</div></div>`)
                .join("");
        } else {
            browsersEl.innerHTML = "<p>No browser data yet</p>";
        }

        const platformsEl = document.getElementById("stats-platforms");
        if (data.platforms && Object.keys(data.platforms).length > 0) {
            platformsEl.innerHTML = Object.entries(data.platforms)
                .map(([name, count]) => `<div class="bar-item"><span class="bar-label">${name}</span><div class="bar-fill" style="width: ${Math.min(count / Math.max(...Object.values(data.platforms)) * 100, 100)}%">${count}</div></div>`)
                .join("");
        } else {
            platformsEl.innerHTML = "<p>No platform data yet</p>";
        }

        const recentEl = document.getElementById("stats-recent");
        if (data.recent_clicks && data.recent_clicks.length > 0) {
            recentEl.innerHTML = `<table class="clicks-table">
                <thead><tr><th>IP</th><th>Country</th><th>City</th><th>Browser</th><th>Time</th></tr></thead>
                <tbody>${data.recent_clicks.slice(0, 20).map(c => `<tr>
                    <td>${c.ip || "-"}</td>
                    <td>${c.country || "-"}</td>
                    <td>${c.city || "-"}</td>
                    <td>${c.user_agent ? c.user_agent.split(" ")[0] : "-"}</td>
                    <td>${c.clicked_at ? new Date(c.clicked_at).toLocaleString() : "-"}</td>
                </tr>`).join("")}</tbody>
            </table>`;
        } else {
            recentEl.innerHTML = "<p>No click data yet</p>";
        }
    } catch (err) {
        section.innerHTML = `<p class="error">Failed to load stats: ${err.message}</p>`;
    }
}

function closeStats() {
    document.getElementById("link-stats-section").style.display = "none";
}

async function handleDelete(shortCode) {
    if (!confirm("Are you sure you want to delete this link?")) return;

    try {
        await deleteLink(shortCode);
        loadLinks();
    } catch (err) {
        alert(err.message);
    }
}

function copyToClipboard(elementId) {
    const text = document.getElementById(elementId).textContent || document.getElementById(elementId).href;
    navigator.clipboard.writeText(text).then(() => {
        const btn = event.target;
        const originalText = btn.textContent;
        btn.textContent = "Copied!";
        setTimeout(() => (btn.textContent = originalText), 2000);
    });
}

async function checkHealthStatus() {
    const pythonEl = document.getElementById("health-python");
    const rustEl = document.getElementById("health-rust");
    const goEl = document.getElementById("health-go");

    pythonEl.classList.add("checking");
    rustEl.classList.add("checking");
    goEl.classList.add("checking");

    const [pythonStatus, rustStatus, goStatus] = await Promise.all([
        checkHealth(),
        checkHealthBackend("rust"),
        checkHealthBackend("go"),
    ]);

    pythonEl.classList.remove("checking");
    pythonEl.classList.add(pythonStatus === "healthy" ? "healthy" : "unhealthy");

    rustEl.classList.remove("checking");
    rustEl.classList.add(rustStatus === "healthy" ? "healthy" : "unhealthy");

    goEl.classList.remove("checking");
    goEl.classList.add(goStatus === "healthy" ? "healthy" : "unhealthy");
}

function switchBackend(backend) {
    API_BASE = `/api/${backend}`;
    document.getElementById("current-backend").textContent =
        backend === "python" ? "Python (FastAPI)" :
        backend === "rust" ? "Rust (Axum)" :
        backend === "go" ? "Go (Gin)" : "Unknown";
}

// Tools
function switchToolTab(tab) {
    document.querySelectorAll(".tool-panel").forEach(p => p.style.display = "none");
    document.querySelectorAll(".tab-btn").forEach(b => b.classList.remove("active"));

    document.getElementById(`tool-${tab}`).style.display = "block";
    event.target.classList.add("active");

    if (tab === "myip") loadMyIP();
    if (tab === "pixels") loadPixels();
}

async function loadMyIP() {
    const el = document.getElementById("ip-info");
    el.innerHTML = "Loading...";
    try {
        const data = await getMyIP();
        el.innerHTML = `
            <div class="ip-card">
                <div class="ip-address">${data.ip}</div>
                ${data.country ? `<div class="ip-detail"><strong>Country:</strong> ${data.country}</div>` : ""}
                ${data.city ? `<div class="ip-detail"><strong>City:</strong> ${data.city}</div>` : ""}
                ${data.isp ? `<div class="ip-detail"><strong>ISP:</strong> ${data.isp}</div>` : ""}
            </div>
        `;
    } catch (err) {
        el.innerHTML = `<p class="error">Failed to load IP info: ${err.message}</p>`;
    }
}

async function handleCheckURL(event) {
    event.preventDefault();
    const url = document.getElementById("check-url-input").value;
    const resultEl = document.getElementById("url-check-result");
    const statusEl = document.getElementById("url-check-status");
    const chainEl = document.getElementById("url-check-chain");
    const warningsEl = document.getElementById("url-check-warnings");

    statusEl.innerHTML = "<p>Checking URL...</p>";
    chainEl.innerHTML = "";
    warningsEl.innerHTML = "";
    resultEl.style.display = "block";

    try {
        const data = await checkUrl(url);
        const safetyClass = data.is_safe ? "safe" : "unsafe";
        const safetyIcon = data.is_safe ? "✓" : "✗";
        statusEl.innerHTML = `
            <div class="url-check-result ${safetyClass}">
                <span class="safety-icon">${safetyIcon}</span>
                <span>${data.is_safe ? "This URL appears safe" : "Caution: This URL may be unsafe"}</span>
            </div>
            <p><strong>Original URL:</strong> ${data.original_url}</p>
            ${data.final_url ? `<p><strong>Final URL:</strong> ${data.final_url}</p>` : ""}
            <p><strong>Redirects:</strong> ${data.total_redirects}</p>
        `;

        if (data.redirect_chain && data.redirect_chain.length > 0) {
            chainEl.innerHTML = "<h4>Redirect Chain:</h4>" +
                data.redirect_chain.map((step, i) =>
                    `<div class="redirect-step"><span class="step-num">${i + 1}</span> ${step.status ? `[${step.status}]` : ""} ${step.url}</div>`
                ).join("");
        }

        if (data.warnings && data.warnings.length > 0) {
            warningsEl.innerHTML = "<h4>Warnings:</h4>" +
                data.warnings.map(w => `<div class="warning-item">⚠ ${w}</div>`).join("");
        }
    } catch (err) {
        statusEl.innerHTML = `<p class="error">Error: ${err.message}</p>`;
    }
}

async function handleUTMBuild(event) {
    event.preventDefault();
    const resultEl = document.getElementById("utm-result");

    try {
        const data = await buildUtm(
            document.getElementById("utm-url").value,
            {
                utm_source: document.getElementById("utm-source").value || undefined,
                utm_medium: document.getElementById("utm-medium").value || undefined,
                utm_campaign: document.getElementById("utm-campaign").value || undefined,
                utm_term: document.getElementById("utm-term").value || undefined,
                utm_content: document.getElementById("utm-content").value || undefined,
            }
        );

        const linkEl = document.getElementById("utm-url-link");
        linkEl.href = data.utm_url;
        linkEl.textContent = data.utm_url;
        resultEl.style.display = "block";
    } catch (err) {
        alert(err.message);
    }
}

async function handleCreatePixel(event) {
    event.preventDefault();
    const nameInput = document.getElementById("pixel-name");
    const name = nameInput.value || null;

    try {
        const data = await createPixel(name);
        const linkEl = document.getElementById("pixel-url-link");
        linkEl.href = data.pixel_url;
        linkEl.textContent = data.pixel_url;
        document.getElementById("pixel-create-result").style.display = "block";
        nameInput.value = "";
        loadPixels();
    } catch (err) {
        alert(err.message);
    }
}

async function loadPixels() {
    const listEl = document.getElementById("pixels-list");
    try {
        const data = await getPixels();
        if (data.pixels.length === 0) {
            listEl.innerHTML = "<p>No pixels created yet.</p>";
            return;
        }
        listEl.innerHTML = data.pixels.map(p => `
            <div class="pixel-item">
                <div class="pixel-info">
                    <strong>${p.code}</strong>
                    ${p.name ? `<span class="pixel-name">${p.name}</span>` : ""}
                    <span class="clicks">${p.clicks} views</span>
                </div>
                <div class="pixel-meta">
                    <code>&lt;img src="${p.pixel_url}" width="1" height="1" /&gt;</code>
                    <button class="delete-btn" onclick="handleDeletePixel('${p.code}')">Delete</button>
                </div>
            </div>
        `).join("");
    } catch (err) {
        listEl.innerHTML = `<p class="error">${err.message}</p>`;
    }
}

async function handleDeletePixel(code) {
    if (!confirm("Delete this tracking pixel?")) return;
    try {
        await deletePixel(code);
        loadPixels();
    } catch (err) {
        alert(err.message);
    }
}

async function handleSetup2FA() {
    showPage("2fa-setup");
    const errorEl = document.getElementById("totp-error");
    errorEl.textContent = "";

    try {
        const data = await setup2FA();
        document.getElementById("totp-secret").value = data.secret;
        document.getElementById("qr-code").innerHTML = `<img src="https://api.qrserver.com/v1/create-qr-code/?data=${encodeURIComponent(data.qr_code_uri)}&size=200x200" alt="QR Code">`;

        const backupEl = document.getElementById("backup-codes");
        const backupList = document.getElementById("backup-codes-list");
        backupList.innerHTML = data.backup_codes.map((c) => `<code>${c}</code>`).join("");
        backupEl.style.display = "block";
    } catch (err) {
        errorEl.textContent = err.message;
    }
}

async function verifyTOTP() {
    const code = document.getElementById("totp-verify-code").value;
    const errorEl = document.getElementById("totp-error");
    errorEl.textContent = "";

    try {
        await verify2FA(code);
        alert("2FA enabled successfully!");
        showPage("dashboard");
    } catch (err) {
        errorEl.textContent = err.message;
    }
}