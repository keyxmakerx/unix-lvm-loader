/** @type {'dashboard' | 'os-picker' | 'themes' | 'luks' | 'backups' | 'logs'} */
let currentPage = $state('dashboard');

/** @type {any} */
let systemOverview = $state(null);

/** @type {boolean} */
let loading = $state(false);

/** @type {string | null} */
let error = $state(null);

/** @type {string | null} */
let notification = $state(null);

export function getAppState() {
  return {
    get currentPage() { return currentPage; },
    set currentPage(v) { currentPage = v; },
    get systemOverview() { return systemOverview; },
    set systemOverview(v) { systemOverview = v; },
    get loading() { return loading; },
    set loading(v) { loading = v; },
    get error() { return error; },
    set error(v) { error = v; },
    get notification() { return notification; },
    set notification(v) { notification = v; },
  };
}
