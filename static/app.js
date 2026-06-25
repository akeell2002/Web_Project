
/* This the mouse glow, will track the cursor position as CSS variables used by the radial-gradient 
   glow on both auth and app pages (body::before in style.css).        */
document.addEventListener('mousemove', function (e) {
    document.body.style.setProperty('--mouse-x', e.clientX + 'px');
    document.body.style.setProperty('--mouse-y', e.clientY + 'px');
});

/* AUTH PAGE FLASH BANNER
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

/* APP TOAST FLASH
   Shows a Bootstrap dismissible alert (top-right) on app pages when
   ?success=<key> is present in the URL. Only runs on .page-app pages
   so it doesn't interfere with auth page flash banners.               */
(function () {
    if (!document.body.classList.contains('page-app')) return;

    var MESSAGES = {
        // For all roles
        'login':                 'Welcome back! You have logged in successfully.',
        'updated':               'Profile updated successfully.',
        'password_reset':        'Password reset successfully. Please log in.',

        // Admin
        'staff_created':         'Staff account created successfully.',
        'doctor_created':        'Doctor account created successfully.',
        'nurse_created':         'Nurse account created successfully.',
        'receptionist_created':  'Receptionist account created successfully.',
        'admin_created':         'Admin account created successfully.',
        'staff_updated':         'Staff account updated successfully.',
        'staff_deleted':         'Staff account deleted successfully.',
        'patient_deleted':       'Patient account deleted successfully.',
        'reply_sent':            'Reply sent and ticket marked as resolved.',

        // Receptionist
        'patient_created':       'Patient account created successfully.',
        'patient_updated':       'Patient details updated successfully.',
        'checked_in':            'Patient checked in successfully.',
        'no_show':               'Patient marked as no-show successfully.',
        'bill_paid':             'Payment collected. Bill marked as paid.',

        // Doctor
        'consultation_saved':    'Consultation saved and bill generated.',
        'admitted':              'Patient admitted, an admission bed has been assigned.',
        'discharged':            'Patient has been discharged successfully.',
        'prescription_saved':    'Prescription saved successfully.',

        // Nurse
        'vitals_saved':          'Patient vitals saved successfully.',
        'logged':                'Medication administeredsuccessfully.',

        // Patient
        'booked':                'Appointment booked successfully. See you soon!',
        'appointment_updated':   'Appointment updated successfully.',
        'appointment_cancelled': 'Appointment cancelled successfully.',
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

    // Populate filter dropdowns from live data
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

// Filter
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

// Sort
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

/* APPOINTMENT SLOT RELOAD 
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

/* CONSULTATION ADMIT TOGGLE
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

/* NURSE TRIAGE VITALS — LIVE RANGE VALIDATION
   Flags a vitals value the moment it exceeds what the column can store
   (so the DB never throws a numeric overflow) and disables that row's
   "Submit Vitals" button while anything is out of range. No-op on pages
   without vitals inputs. */
// Single source of truth for triage vitals limits — tweak these values here.
// Numeric fields use {min, max}; text fields (BP) use {maxLength, pattern}.
var VITALS_LIMITS = {
    'temperature':    { min: 0, max: 50.00,  label: 'Temperature' },
    'weight_kg':      { min: 0, max: 999.99, label: 'Weight' },
    'height_cm':      { min: 0, max: 999.99, label: 'Height' },
    'blood_pressure': { maxLength: 20, pattern: /^\d{1,3}\s*\/\s*\d{1,3}$/, label: 'BP' }
};

// Returns an error message if the vitals input is out of range, else ''.
function vitalsError(input) {
    var rule = VITALS_LIMITS[input.name];
    if (!rule) return '';

    var v = input.value.trim();
    if (v === '') return '';

    // Text field (e.g. BP): length + format.
    if (rule.maxLength != null || rule.pattern) {
        if (rule.maxLength != null && v.length > rule.maxLength) {
            return rule.label + ' max ' + rule.maxLength + ' characters';
        }
        if (rule.pattern && !rule.pattern.test(v)) {
            return 'Use format like 120/80';
        }
        return '';
    }

    // Numeric field.
    var num = parseFloat(v);
    if (isNaN(num)) return rule.label + ' looks invalid';
    if (rule.max != null && num > rule.max) return rule.label + ' max is ' + rule.max;
    if (rule.min != null && num < rule.min) return rule.label + ' must be ' + rule.min + ' or more';
    return '';
}

function validateVitalsInput(input) {
    if (!VITALS_LIMITS[input.name]) return;

    var message = vitalsError(input);
    var err     = input.parentNode.querySelector('.vitals-error');

    if (message) {
        input.style.borderColor = '#DC2626';
        if (!err) {
            err = document.createElement('div');
            err.className = 'vitals-error';
            err.style.cssText = 'color:#DC2626; font-size:11px; font-weight:600; margin-top:3px;';
            input.parentNode.appendChild(err);
        }
        err.textContent = message;
        err.style.display = 'block';
    } else {
        input.style.borderColor = '';
        if (err) { err.style.display = 'none'; }
    }

    // Disable this row's submit button while any vitals field is out of range.
    var formEl = input.closest('form');
    if (!formEl) return;
    var anyInvalid = Array.prototype.slice
        .call(formEl.querySelectorAll('.vitals-input'))
        .some(function (i) { return vitalsError(i) !== ''; });
    var btn = formEl.querySelector('button[type="submit"]');
    if (btn) {
        btn.disabled      = anyInvalid;
        btn.style.opacity = anyInvalid ? '0.5' : '';
        btn.style.cursor  = anyInvalid ? 'not-allowed' : '';
    }
}

document.addEventListener('DOMContentLoaded', function () {
    document.querySelectorAll('.vitals-input').forEach(function (input) {
        var rule = VITALS_LIMITS[input.name];
        if (rule) {
            // Apply the limits set above to the input natively, so the form
            // and the live check always follow app.js (the single source).
            if (rule.min != null)       { input.min = rule.min; }
            if (rule.max != null)       { input.max = rule.max; }
            if (rule.maxLength != null) { input.maxLength = rule.maxLength; }
        }
        input.addEventListener('input', function () { validateVitalsInput(input); });
    });
});