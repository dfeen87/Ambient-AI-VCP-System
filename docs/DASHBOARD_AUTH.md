# Dashboard Authentication Integration

## Overview

The dashboard has been successfully updated to work with the new authentication and node ownership system.

## Changes Made

### 1. Authentication UI

**Login Form:**
- Username field
- Password field  
- Submit button
- Tab-based UI to switch between login and registration

**Registration Form:**
- Username field (3-32 characters)
- Password field (minimum 8 characters)
- Automatic login after successful registration
- API key displayed to user (one-time view)

### 2. Authentication Flow

```
┌─────────────────┐
│ User visits     │
│ dashboard       │
└────────┬────────┘
         │
    ┌────▼─────────────────┐
    │ Check localStorage   │
    │ for auth token      │
    └────┬────────┬────────┘
         │        │
    No   │        │  Yes
         │        │
    ┌────▼─────┐  │
    │ Show     │  │
    │ Login/   │  │
    │ Register │  │
    └──────────┘  │
                  │
            ┌─────▼────────┐
            │ Show Main    │
            │ Dashboard    │
            │ Content      │
            └──────────────┘
```

### 3. Token Management

**Storage:**
- JWT token stored in `localStorage` with key `vcp_auth_token`
- Username stored in `localStorage` with key `vcp_username`

**Usage:**
- All API requests include `Authorization: Bearer <token>` header
- Token automatically included via `getAuthHeaders()` function

**Lifecycle:**
- Token persists across browser sessions
- Logout removes token from localStorage
- Missing/invalid token redirects to login

### 4. JavaScript Functions Added

**Authentication:**
- `checkAuth()` - Verify login status on page load
- `handleLogin(e)` - Process login form submission
- `handleRegister(e)` - Process registration form submission
- `logout()` - Clear auth data and return to login
- `getAuthHeaders()` - Return headers with auth token

**UI Management:**
- `showAuthTab(tab)` - Switch between login/register tabs

### 5. Updated API Calls

**Node Registration:**
```javascript
// Before (❌ No auth):
fetch(`${apiBaseUrl}/api/v1/nodes`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
});

// After (✅ With auth):
fetch(`${apiBaseUrl}/api/v1/nodes`, {
    method: 'POST',
    headers: getAuthHeaders(), // Includes Authorization header
    body: JSON.stringify(body),
});
```

### 6. CSS Styling

**New Styles:**
- `.auth-tabs` - Tab container
- `.auth-tab` - Individual tab buttons
- `.auth-tab.active` - Active tab highlighting
- `.auth-form` - Form containers with proper spacing

## User Experience

### First-Time User

1. Visits dashboard → sees login/register tabs
2. Clicks "Register" tab
3. Enters username (e.g., "alice")
4. Enters password (min 8 chars)
5. Clicks "Register" button
6. Sees toast message with API key (save this!)
7. Automatically logged in
8. Dashboard loads with full functionality

### Returning User

1. Visits dashboard
2. Automatically logged in (token in localStorage)
3. Dashboard shows immediately
4. User info shown in header: "Logged in as: alice"
5. Logout button available

### Session Management

- Token persists across browser tabs
- Token persists after browser restart
- Logout clears token completely
- Re-login required after logout

## Security Features

1. **No Plaintext Passwords:** Passwords sent securely over HTTPS
2. **Token Storage:** JWT stored in localStorage (not cookies)
3. **Auto-Expiration:** Tokens expire after 24 hours (server-side)
4. **Clean Logout:** Token completely removed from browser
5. **Protected Routes:** Dashboard requires valid token

## Testing Checklist

### Manual Testing

- [ ] **Registration Flow**
  - [ ] Register new user with valid username/password
  - [ ] Verify API key displayed in toast message
  - [ ] Verify auto-login after registration
  - [ ] Try registering with duplicate username (should fail)
  - [ ] Try short password (should show validation error)

- [ ] **Login Flow**
  - [ ] Login with valid credentials
  - [ ] Verify dashboard loads after login
  - [ ] Verify user info shows in header
  - [ ] Try login with invalid credentials (should fail)
  - [ ] Verify error message for failed login

- [ ] **Token Persistence**
  - [ ] Login and close browser
  - [ ] Reopen browser and visit dashboard
  - [ ] Verify still logged in
  - [ ] Check localStorage for token

- [ ] **Logout Flow**
  - [ ] Click logout button
  - [ ] Verify redirected to login screen
  - [ ] Check localStorage (token should be gone)
  - [ ] Try accessing dashboard (should show login)

- [ ] **Node Registration** 
  - [ ] Register node while logged in
  - [ ] Verify node appears in list
  - [ ] Try registering without login (should see login screen)

### Browser Testing

- [ ] Chrome/Edge
- [ ] Firefox
- [ ] Safari
- [ ] Mobile browsers

## Known Limitations

1. **No Delete Buttons Yet:** Node delete functionality not added to UI (API exists)
2. **No Heartbeat Button:** Heartbeat update button not added to UI (API exists)
3. **No Owner Filter:** Cannot filter to show only user's nodes
4. **No Token Refresh:** Token expires after 24h, requires re-login
5. **No Remember Me:** No persistent login option beyond localStorage

## Future Enhancements

### Phase 1 (High Priority)
- [ ] Add delete button for each node (only shown for owned nodes)
- [ ] Add heartbeat update button
- [ ] Show node owner in node list
- [ ] Add "My Nodes" filter toggle

### Phase 2 (Medium Priority)
- [ ] Token auto-refresh before expiration
- [ ] "Remember me" option for extended sessions
- [ ] Password strength indicator
- [ ] Forgot password flow
- [ ] Email verification

### Phase 3 (Low Priority)
- [ ] Multi-factor authentication
- [ ] Session management (view/revoke active sessions)
- [ ] Activity log
- [ ] Account settings page

## API Key Security

**Important:** After registration, users receive an API key:
```json
{
  "user_id": "...",
  "username": "alice",
  "api_key": "vcp_Abc123...",
  "message": "User registered successfully. Save your API key - it won't be shown again."
}
```

**User Guidance:**
- API key shown only once
- Store in password manager
- Can be used for service-to-service auth
- Alternative to JWT for automated clients

## Troubleshooting

### "Unable to reach API" Error
1. Check if API server is running
2. Verify API URL in dashboard header
3. Check browser console for CORS errors
4. Try clicking "Connect" button

### Login Fails Despite Correct Credentials
1. Check browser console for errors
2. Verify API URL is correct
3. Check if server is in production mode
4. Verify JWT_SECRET is configured

### Token Expired
- Tokens expire after 24 hours
- User must login again
- Future: Implement token refresh

### Cannot Register Node
1. Verify logged in (user info shown in header)
2. Check browser console for 401 errors
3. Try logging out and back in
4. Check if token is in localStorage

## Developer Notes

### Adding Delete Button to UI

To add delete functionality:

```javascript
// In renderNodes function, add delete button:
<button onclick="deleteNode('${n.node_id}')" class="btn btn-ghost">Delete</button>

// Add delete function:
async function deleteNode(nodeId) {
    if (!confirm(`Delete node ${nodeId}?`)) return;
    
    try {
        const res = await fetch(`${apiBaseUrl}/api/v1/nodes/${nodeId}`, {
            method: 'DELETE',
            headers: getAuthHeaders(),
        });
        
        if (res.ok) {
            showToast('Node deleted successfully');
            fetchData();
        } else {
            const err = await res.json();
            showToast(err.message || 'Delete failed', true);
        }
    } catch (err) {
        showToast('Network error: ' + err.message, true);
    }
}
```

### Adding Heartbeat Button

```javascript
async function sendHeartbeat(nodeId) {
    try {
        const res = await fetch(`${apiBaseUrl}/api/v1/nodes/${nodeId}/heartbeat`, {
            method: 'PUT',
            headers: getAuthHeaders(),
        });
        
        if (res.ok) {
            showToast('Heartbeat sent');
            fetchData();
        } else {
            const err = await res.json();
            showToast(err.message || 'Heartbeat failed', true);
        }
    } catch (err) {
        showToast('Network error: ' + err.message, true);
    }
}
```

## Conclusion

The dashboard is now fully integrated with the authentication system and ready for use. Users must login to access the dashboard, and all API calls include proper authorization headers. The node registration endpoint now correctly links nodes to the authenticated user's account.
