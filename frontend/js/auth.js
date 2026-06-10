let tempLoginToken = null;

async function handleRegister(event) {
    event.preventDefault();
    const name = document.getElementById("register-name").value;
    const email = document.getElementById("register-email").value;
    const password = document.getElementById("register-password").value;
    const errorEl = document.getElementById("register-error");
    errorEl.textContent = "";

    try {
        const data = await register(email, password, name);
        localStorage.setItem("access_token", data.access_token);
        localStorage.setItem("refresh_token", data.refresh_token);
        localStorage.setItem("user_email", email);
        showPage("dashboard");
        loadLinks();
    } catch (err) {
        errorEl.textContent = err.message;
    }
}

async function handleLogin(event) {
    event.preventDefault();
    const email = document.getElementById("login-email").value;
    const password = document.getElementById("login-password").value;
    const errorEl = document.getElementById("login-error");
    errorEl.textContent = "";

    try {
        const data = await login(email, password);

        if (data.totp_required) {
            document.getElementById("login-2fa").style.display = "block";
            tempLoginToken = data.access_token || data.refresh_token;
            const totpCode = document.getElementById("login-totp").value;
            if (totpCode) {
                await complete2FALogin(totpCode);
            } else {
                document.getElementById("login-totp").addEventListener("input", async function handler() {
                    if (this.value.length === 6) {
                        this.removeEventListener("input", handler);
                        await complete2FALogin(this.value);
                    }
                });
            }
            return;
        }

        localStorage.setItem("access_token", data.access_token);
        localStorage.setItem("refresh_token", data.refresh_token);
        localStorage.setItem("user_email", email);
        showPage("dashboard");
        loadLinks();
    } catch (err) {
        errorEl.textContent = err.message;
    }
}

async function complete2FALogin(code) {
    const errorEl = document.getElementById("login-error");
    try {
        const data = await loginWith2FA(code, tempLoginToken);
        localStorage.setItem("access_token", data.access_token);
        localStorage.setItem("refresh_token", data.refresh_token);
        localStorage.setItem("user_email", document.getElementById("login-email").value);
        showPage("dashboard");
        loadLinks();
    } catch (err) {
        errorEl.textContent = err.message;
    }
}

function logout() {
    localStorage.removeItem("access_token");
    localStorage.removeItem("refresh_token");
    localStorage.removeItem("user_email");
    showPage("login");
}

function isAuthenticated() {
    return !!localStorage.getItem("access_token");
}

function getCurrentUser() {
    return localStorage.getItem("user_email");
}
