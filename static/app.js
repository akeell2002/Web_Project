
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
        'patient_created':       'Patient account created successfully.',
        'booked':                'Appointment booked successfully. See you soon!',
        'appointment_updated':   'Appointment updated successfully.',
        'appointment_cancelled': 'Appointment cancelled successfully.',
        'checked_in':            'Patient checked in successfully.',
        'admitted':              'Patient admitted, an admission bed has been assigned.',
        'consultation_saved':    'Consultation saved and bill generated.',
        'discharged':            'Patient discharged successfully. Bed is now free.',
        'staff_updated':         'Staff account updated successfully.',
        'staff_deleted':         'Staff account deleted successfully.',
        'reply_sent':            'Reply sent and ticket marked as resolved.',
        'no_show':               'Patient marked as no-show successfully.',
        'doctor_created':        'Doctor account created successfully.',
        'nurse_created':         'Nurse account created successfully.',
        'receptionist_created':  'Receptionist account created successfully.',
        'admin_created':         'Admin account created successfully.',
        'staff_created':         'Staff account created successfully.',
        'patient_created':       'Patient account created successfully.',
        'booked':                'Appointment booked successfully. See you soon!',
        'appointment_updated':   'Appointment updated successfully.',
        'appointment_cancelled': 'Appointment cancelled successfully.',
        'vitals_saved':          'Patient vitals saved successfully.',
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

(function () {
    var tbody        = document.getElementById('sec-tbody');
    if (!tbody) return; // Security-log page only; skip on every other page.
    var rows         = Array.from(tbody.querySelectorAll('tr:not(#sec-empty-row)'));
    var originalOrder = rows.slice(); // snapshot original DOM order
    var countEl = document.getElementById('sec-count');
    var noRes   = document.getElementById('sec-no-results');
    var searchEl = document.getElementById('sec-search');
    var actionEl = document.getElementById('sec-action');
    var roleEl   = document.getElementById('sec-role');
    var clearBtn = document.getElementById('sec-clear');

    // ── Populate filter dropdowns from live data ──────────────────
    // Normalise to lowercase keys to avoid Admin/admin duplicates.
    var actions = {}, roles = {};
    rows.forEach(function (row) {
        var cells  = row.querySelectorAll('td');
        var action = cells[1].textContent.trim();
        var role   = cells[3].querySelector('span') ? cells[3].querySelector('span').textContent.trim() : '';
        if (action) actions[action.toLowerCase()] = action;
        if (role)   roles[role.toLowerCase()]     = role.charAt(0).toUpperCase() + role.slice(1).toLowerCase();
    });
    // Also write the normalised lowercase back into each row's role span
    // so filtering matches correctly against the normalised value.
    rows.forEach(function (row) {
        var span = row.querySelectorAll('td')[3].querySelector('span');
        if (span) span.textContent = span.textContent.trim().toLowerCase();
    });
    Object.keys(actions).sort().forEach(function (key) {
        var opt = document.createElement('option');
        opt.value = key; opt.textContent = actions[key];
        actionEl.appendChild(opt);
    });
    Object.keys(roles).sort().forEach(function (key) {
        var opt = document.createElement('option');
        opt.value = key; opt.textContent = roles[key];
        roleEl.appendChild(opt);
    });

// ── Filter ────────────────────────────────────────────────────
function applyFilters() {
    var q      = searchEl.value.trim().toLowerCase();
    var action = actionEl.value;
    var role   = roleEl.value;
    var visible = 0;

    rows.forEach(function (row) {
        var cells     = row.querySelectorAll('td');
        var text      = row.textContent.toLowerCase();
        var rowAction = cells[1].textContent.trim().toLowerCase();
        var rowRole   = cells[3].querySelector('span') ? cells[3].querySelector('span').textContent.trim().toLowerCase() : '';

        var show = (!q || text.includes(q))
                && (!action || rowAction === action)
                && (!role   || rowRole   === role);

        row.style.display = show ? '' : 'none';
        if (show) visible++;
    });

    var word = visible === 1 ? 'event' : 'events';
    countEl.textContent = visible + ' ' + word + ' found';
    noRes.style.display = (visible === 0 && rows.length > 0) ? 'block' : 'none';
}

searchEl.addEventListener('input',  applyFilters);
actionEl.addEventListener('change', applyFilters);
roleEl.addEventListener('change',   applyFilters);
clearBtn.addEventListener('click', function () {
    searchEl.value = '';
    actionEl.selectedIndex = 0;
    roleEl.selectedIndex = 0;
    // Restore original row order
    originalOrder.forEach(function (r) { tbody.appendChild(r); });
    rows = originalOrder.slice();
    // Reset sort arrows
    sortState.col = -1; sortState.asc = true;
    document.querySelectorAll('.sec-sortable .sec-arrow').forEach(function (a) { a.textContent = '↕'; });
    applyFilters();
});

// ── Sort ──────────────────────────────────────────────────────
var sortState = { col: -1, asc: true };

document.querySelectorAll('.sec-sortable').forEach(function (th) {
    th.addEventListener('click', function () {
        var col = parseInt(th.dataset.col);
        if (sortState.col === col) {
            sortState.asc = !sortState.asc;
        } else {
            sortState.col = col;
            sortState.asc = true;
        }

        // Update arrow indicators
        document.querySelectorAll('.sec-sortable .sec-arrow').forEach(function (a) { a.textContent = '↕'; });
        th.querySelector('.sec-arrow').textContent = sortState.asc ? '↑' : '↓';

        rows.sort(function (a, b) {
            var aText = a.querySelectorAll('td')[col].textContent.trim().toLowerCase();
            var bText = b.querySelectorAll('td')[col].textContent.trim().toLowerCase();
            if (aText < bText) return sortState.asc ? -1 :  1;
            if (aText > bText) return sortState.asc ?  1 : -1;
            return 0;
        });

        rows.forEach(function (r) { tbody.appendChild(r); });
        applyFilters();
    });
});
}());

        // Tab switching for doctor consultation page
function switchTab(id) {
    ['tab-general','tab-history','tab-rx'].forEach(t => {
        document.getElementById(t).style.display = t === id ? 'block' : 'none';
    });
    const btnMap = { 'tab-general':'btn-tab-general', 'tab-history':'btn-tab-history', 'tab-rx':'btn-tab-rx' };
    Object.entries(btnMap).forEach(([tab, btn]) => {
        const el = document.getElementById(btn);
        const active = tab === id;
        el.style.fontWeight    = active ? '600' : '500';
        el.style.color         = active ? '#1E3A8A' : '#6B7280';
        el.style.background    = active ? 'white' : 'transparent';
        el.style.borderBottom  = active ? '2px solid #1E3A8A' : '2px solid transparent';
    });
}

function toggleCard(id) {
    const card = document.getElementById('card-' + id);
    card.classList.toggle('open');
}

function updateFormAction(patientId) {
    const select = document.getElementById('appt-select-' + patientId);
    const form = document.getElementById('form-' + patientId);
    const appointmentId = select.value;
    form.action = '/staff/doctor/prescribe/' + appointmentId;
}

function filterPatients() {
    const query = document.getElementById('patient-search').value.toLowerCase().trim();
    const cards = document.querySelectorAll('#appt-list .appt-card');
    let visibleCount = 0;

    cards.forEach(card => {
    const name = card.querySelector('.appt-patient').textContent.toLowerCase();
    const match = name.includes(query);
    card.style.display = match ? '' : 'none';
    if (match) visibleCount++;
    });
    document.getElementById('no-results').style.display = visibleCount === 0 ? 'block' : 'none';
}

// Filter patient at patient directory page
function filterPatients(query) {
   const q = query.toLowerCase();
   const rows = document.querySelectorAll('#patient-table tbody tr[data-searchable]');
   let visible = 0;
   rows.forEach(row => {
   const text = row.getAttribute('data-searchable');
   if (text.includes(q)) { row.style.display = ''; visible++; }
   else { row.style.display = 'none'; }
   });
   document.getElementById('no-results').style.display = (visible === 0 && q.length > 0) ? 'block' : 'none';
}

function filterRows(filter, btn) {
    document.querySelectorAll('.bm-tab').forEach(t => t.classList.remove('active'));
    btn.classList.add('active');

    const rows = document.querySelectorAll('#patientTableBody tr');
    rows.forEach(row => {
    const status = row.dataset.status || '';
    const show = filter === 'all'
    || (filter === 'waiting' && status === 'checked_in')
    || (filter === 'vitals_taken' && status === 'vitals_taken')
    || (filter === 'completed' && status === 'completed');
    row.style.display = show ? '' : 'none';
    });
}

// Search
function searchTable(q) {
    const query = q.toLowerCase();
    document.querySelectorAll('#patientTableBody tr').forEach(row => {
    const name = row.dataset.name || '';
    const room = row.dataset.room || '';
    row.style.display = (name.includes(query) || room.includes(query)) ? '' : 'none';
});

// Also filter bed cards if visible
    document.querySelectorAll('.bed-card').forEach(card => {
    const name = card.dataset.name || '';
    const patient = card.dataset.patient || '';
    card.style.display = (name.includes(query) || patient.includes(query)) ? '' : 'none';
    });
}

/* ── APPOINTMENT SLOT RELOAD ─────────────────────────────────────────
   Reloads the page with the chosen doctor / date / visit type so the
   backend can render available time slots. Shared by the booking page
   and the reschedule page — if a hidden #appointment_id field is present
   we target the reschedule (edit) route, otherwise the booking route.   */
function reloadAvailableSlots() {
    const docId   = document.getElementById("doctor_select").value;
    const dateVal = document.getElementById("date_select").value;
    const visitDd = document.getElementById("visit_type");

    if (!docId || !dateVal || !visitDd.value) return;

    const durationVal  = visitDd.options[visitDd.selectedIndex].getAttribute("data-duration");
    const visitTypeVal = visitDd.value;
    const apptEl       = document.getElementById("appointment_id");

    const base = apptEl
        ? `/patient/appointments/${apptEl.value}/edit`
        : `/patient/appointments/book`;

    window.location.href =
        `${base}?doctor_id=${docId}&date=${dateVal}&duration_minutes=${durationVal}&visit_type=${visitTypeVal}`;
}

/* ── CONSULTATION ADMIT TOGGLE ───────────────────────────────────────
   On the doctor consultation page, the "Need to admit?" Yes/No choice
   enables exactly one of the two submit buttons. No-op on other pages.  */
function updateAdmitButtons() {
    var btnAdmit = document.getElementById('btn-admit');
    var btnSign  = document.getElementById('btn-sign');
    if (!btnAdmit || !btnSign) return;

    var choice = document.querySelector('input[name="admit_choice"]:checked');
    var admit  = choice && choice.value === 'yes';

    btnAdmit.disabled = !admit;
    btnSign.disabled  = admit;

    btnAdmit.style.opacity = admit ? '1' : '0.45';
    btnAdmit.style.cursor  = admit ? 'pointer' : 'not-allowed';
    btnSign.style.opacity  = admit ? '0.45' : '1';
    btnSign.style.cursor   = admit ? 'not-allowed' : 'pointer';
}
// Set the initial enabled/disabled state once the page has loaded.
document.addEventListener('DOMContentLoaded', updateAdmitButtons);