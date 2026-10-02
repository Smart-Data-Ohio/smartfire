// Rails d7c7de92: installed Capybara default and the original explicit waits.
// Positive checks and discrimination probes must share these deadlines.
export const CAPYBARA_DEFAULT=2000;
export const DELIVERY_WAIT=10000;
export const CABLE_WAIT=15000;
export const REVIEW_GROUPS=new Set(['message_interactions','message_actions_mobile','message_toolbar','code_highlighting']);
