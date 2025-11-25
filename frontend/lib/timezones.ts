export const TIMEZONES: string[] = (() => {
  try {
    // @ts-ignore - supportedValuesOf is available in modern environments
    if (typeof Intl !== 'undefined' && Intl.supportedValuesOf) {
      // @ts-ignore
      return Intl.supportedValuesOf('timeZone');
    }
  } catch (e) {
    console.error('Intl.supportedValuesOf not supported', e);
  }

  // Fallback for older environments
  return [
    'UTC',
    'Europe/London',
    'Europe/Stockholm',
    'Europe/Paris',
    'Europe/Berlin',
    'America/New_York',
    'America/Los_Angeles',
    'America/Chicago',
    'America/Toronto',
    'Asia/Tokyo',
    'Asia/Ho_Chi_Minh_City',
    'Asia/Singapore',
    'Asia/Shanghai',
    'Asia/Seoul',
    'Australia/Sydney',
    'Australia/Melbourne',
    'Pacific/Auckland',
  ];
})();
