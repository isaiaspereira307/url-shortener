let API_BASE = "/api/python";

let isRefreshing = false;
let refreshSubscribers = [];

function subscribeTokenRefresh(cb) {
    refreshSubscribers.push(cb);
}

function onTokenRefreshed(newToken) {
    refreshSubscribers.forEach(cb => cb(newToken));
    refreshSubscribers = [];
}

async function refreshToken() {
    const refreshTokenValue = localStorage.getItem("refresh_token");
    if (!refreshTokenValue) {
        logout();
        throw new Error("No refresh token");
    }

    if (isRefreshing) {
        return new Promise(resolve => {
            subscribeTokenRefresh(token => resolve(token));
        });
    }

    isRefreshing = true;
    try {
        const response = await fetch(`${API_BASE}/auth/refresh`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ refresh_token: refreshTokenValue }),
        });

        if (!response.ok) {
            logout();
            throw new Error("Session expired");
        }

        const data = await response.json();
        localStorage.setItem("access_token", data.access_token);
        if (data.refresh_token) {
            localStorage.setItem("refresh_token", data.refresh_token);
        }
        onTokenRefreshed(data.access_token);
        return data.access_token;
    } catch (err) {
        logout();
        throw err;
    } finally {
        isRefreshing = false;
    }
}

async function apiRequest(path, options = {}) {
    const headers = {
        "Content-Type": "application/json",
        ...options.headers,
    };

    const token = localStorage.getItem("access_token");
    if (token) {
        headers["Authorization"] = `Bearer ${token}`;
    }

    let response = await fetch(`${API_BASE}${path}`, {
        ...options,
        headers,
    });

    if (response.status === 401 && localStorage.getItem("refresh_token")) {
        const newToken = await refreshToken();
        headers["Authorization"] = `Bearer ${newToken}`;
        response = await fetch(`${API_BASE}${path}`, {
            ...options,
            headers,
        });
    }

    if (!response.ok) {
        const error = await response.json().catch(() => ({ error: "Request failed" }));
        throw new Error(error.detail || error.error || "Request failed");
    }

    if (response.status === 204) return null;
    return response.json();
}

async function register(email, password, tenantName) {
    return apiRequest("/auth/register", {
        method: "POST",
        body: JSON.stringify({ email, password, tenant_name: tenantName }),
    });
}

async function login(email, password) {
    return apiRequest("/auth/login", {
        method: "POST",
        body: JSON.stringify({ email, password }),
    });
}

async function loginWith2FA(totpCode, tempToken) {
    return apiRequest("/auth/login/2fa", {
        method: "POST",
        headers: { Authorization: `Bearer ${tempToken}` },
        body: JSON.stringify({ code: totpCode }),
    });
}

async function shortenUrl(url) {
    return apiRequest("/shorten", {
        method: "POST",
        body: JSON.stringify({ url }),
    });
}

async function getLinks(page = 1, limit = 20, sort = "created_at", order = "desc") {
    return apiRequest(`/links?page=${page}&limit=${limit}&sort=${sort}&order=${order}`);
}

async function deleteLink(shortCode) {
    return apiRequest(`/links/${shortCode}`, { method: "DELETE" });
}

async function getLinkStats(shortCode) {
    return apiRequest(`/links/${shortCode}/stats`);
}

async function getMyIP() {
    return apiRequest("/myip");
}

async function checkUrl(url) {
    return apiRequest("/check-url", {
        method: "POST",
        body: JSON.stringify({ url }),
    });
}

async function createPixel(name) {
    return apiRequest("/pixel", {
        method: "POST",
        body: JSON.stringify({ name }),
    });
}

async function getPixels(page = 1, limit = 20) {
    return apiRequest(`/pixel?page=${page}&limit=${limit}`);
}

async function deletePixel(code) {
    return apiRequest(`/pixel/${code}`, { method: "DELETE" });
}

async function buildUtm(url, params) {
    return apiRequest("/utm-builder", {
        method: "POST",
        body: JSON.stringify({ url, ...params }),
    });
}

async function setup2FA() {
    return apiRequest("/auth/2fa/setup", { method: "POST" });
}

async function verify2FA(code) {
    return apiRequest("/auth/2fa/verify", {
        method: "POST",
        body: JSON.stringify({ code }),
    });
}

async function checkHealth() {
    try {
        const response = await fetch(`${API_BASE}/health`);
        return response.ok ? "healthy" : "unhealthy";
    } catch {
        return "unhealthy";
    }
}

async function checkHealthBackend(backend) {
    try {
        const response = await fetch(`/api/${backend}/health`);
        return response.ok ? "healthy" : "unhealthy";
    } catch {
        return "unhealthy";
    }
}