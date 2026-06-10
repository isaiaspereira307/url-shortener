import http from 'k6/http';
import { check, sleep } from 'k6';
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.1/index.js';

export const options = {
  stages: [
    { duration: '30s', target: 50 },
    { duration: '1m', target: 100 },
    { duration: '30s', target: 0 },
  ],
  thresholds: {
    http_req_duration: ['p(95)<200'],
    http_req_failed: ['rate<0.05'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost';
const BACKEND = __ENV.BACKEND || 'python';

let authToken = '';
let refreshToken = '';

export function setup() {
  const registerPayload = JSON.stringify({
    email: `benchmark-${Date.now()}@test.com`,
    password: 'benchmarkpass123',
    tenant_name: 'Benchmark Tenant',
  });

  const registerRes = http.post(
    `${BASE_URL}/api/${BACKEND}/auth/register`,
    registerPayload,
    { headers: { 'Content-Type': 'application/json' } }
  );

  check(registerRes, {
    'register status is 201 or 409': (r) => r.status === 201 || r.status === 409,
  });

  const loginPayload = JSON.stringify({
    email: `benchmark-${Date.now() - 1000}@test.com`,
    password: 'benchmarkpass123',
  });

  const loginRes = http.post(
    `${BASE_URL}/api/${BACKEND}/auth/login`,
    loginPayload,
    { headers: { 'Content-Type': 'application/json' } }
  );

  if (loginRes.status === 200) {
    const body = loginRes.json();
    authToken = body.access_token;
    refreshToken = body.refresh_token;
  }

  return { authToken, refreshToken };
}

export default function (data) {
  const headers = {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${data.authToken}`,
  };

  const shortenPayload = JSON.stringify({
    url: `https://example.com/benchmark/${Date.now()}`,
  });

  const shortenRes = http.post(
    `${BASE_URL}/api/${BACKEND}/shorten`,
    shortenPayload,
    { headers }
  );

  check(shortenRes, {
    'shorten status is 201': (r) => r.status === 201,
    'shorten has short_url': (r) => {
      try {
        return r.json().short_url !== undefined;
      } catch {
        return false;
      }
    },
  });

  const linksRes = http.get(
    `${BASE_URL}/api/${BACKEND}/links`,
    { headers }
  );

  check(linksRes, {
    'list links status is 200': (r) => r.status === 200,
  });

  const healthRes = http.get(`${BASE_URL}/api/${BACKEND}/health`);
  check(healthRes, {
    'health status is 200': (r) => r.status === 200,
  });

  sleep(1);
}

export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    [`benchmark-${BACKEND}-${Date.now()}.json`]: JSON.stringify(data),
  };
}
