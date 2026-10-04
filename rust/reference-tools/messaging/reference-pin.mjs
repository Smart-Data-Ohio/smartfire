import {readFileSync} from 'node:fs';

export const PIN=readFileSync(new URL('../../parity/reference.sha',import.meta.url),'utf8').trim();
export const REFERENCE_IMAGE=process.env.PARITY_IMAGE||'campfire-reference';
