
/* This the mouse glow, will track the cursor position as CSS variables used by the radial-gradient 
   glow on both auth and app pages (body::before in style.css).        */
document.addEventListener('mousemove', function (e) {
    document.body.style.setProperty('--mouse-x', e.clientX + 'px');
    document.body.style.setProperty('--mouse-y', e.clientY + 'px');
});

/* ── AUTH PAGE FLASH BANNER ─────────────────────────────────────────
   Reads URL params and shows a message inside #auth-flash on auth
   pages (patient/login, staff/login). Messages are defined via the
   element's data-messages attribute (JSON: "param=value" → message). */
(function () {
    var el = document.getElementById('auth-flash');
    if (!el) return;

    var messages = {};
    try { messages = JSON.parse(el.dataset.messages || '{}'); } catch (e) { return; }

    var params = new URLSearchParams(window.location.search);
    for (var key in messages) {
        var parts = key.split('=');
        if (params.get(parts[0]) === parts[1]) {
            el.textContent = '✓ ' + messages[key];
            el.style.display = 'block';
            // Auto-fade after 6 seconds
            setTimeout(function () {
                el.style.transition = 'opacity 0.5s';
                el.style.opacity = '0';
                setTimeout(function () { el.style.display = 'none'; }, 500);
            }, 6000);
            // Clean URL so refresh doesn't re-trigger
            history.replaceState(null, '', window.location.pathname);
            break;
        }
    }
}());

/* ── APP TOAST FLASH ────────────────────────────────────────────────
   Shows a Bootstrap dismissible alert (top-right) on app pages when
   ?success=<key> is present in the URL. Only runs on .page-app pages
   so it doesn't interfere with auth page flash banners.               */
(function () {
    if (!document.body.classList.contains('page-app')) return;

    var MESSAGES = {
        'login':                 'Welcome back! You have logged in successfully.',
        'updated':               'Profile updated successfully.',
        'doctor_created':        'Doctor account created successfully.',
        'nurse_created':         'Nurse account created successfully.',
        'receptionist_created':  'Receptionist account created successfully.',
        'admin_created':         'Admin account created successfully.',
        'staff_created':         'Staff account created successfully.',
    };

    var params = new URLSearchParams(window.location.search);
    var key    = params.get('success');
    if (!key || !MESSAGES[key]) return;

    var toast = document.createElement('div');
    toast.className  = 'alert alert-success alert-dismissible fade show';
    toast.setAttribute('role', 'alert');
    toast.style.cssText = [
        'position:fixed', 'top:70px', 'right:24px', 'z-index:9999',
        'min-width:280px', 'max-width:420px',
        'box-shadow:0 4px 20px rgba(0,0,0,0.15)'
    ].join(';');
    toast.innerHTML = MESSAGES[key] +
        '<button type="button" class="btn-close" data-bs-dismiss="alert" aria-label="Close"></button>';

    document.body.appendChild(toast);

    setTimeout(function () {
        toast.classList.remove('show');
        setTimeout(function () { toast.remove(); }, 300);
    }, 5000);

    history.replaceState(null, '', window.location.pathname);
}());
