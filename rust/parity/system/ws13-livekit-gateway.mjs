// Unchanged gateway implementation; only its local test listener/callback differ.
import { writeFile } from 'node:fs/promises';
import { createGateway } from '../../../script/livekit-gateway/gateway.mjs';
const [campfireUrl,portFile]=process.argv.slice(2);
let gateway;
for(let port=52300;port<=52349;port++) {
  const candidate=createGateway({listenHost:'127.0.0.1',listenPort:port,campfireUrl,internalUrl:process.env.LIVEKIT_INTERNAL_URL,gatewaySecret:process.env.LIVEKIT_GATEWAY_SECRET,livekitApiKey:process.env.LIVEKIT_API_KEY,livekitApiSecret:process.env.LIVEKIT_API_SECRET,
    onDecision:({type})=>console.log(`WS13 gateway decision: ${type}`),
    onFatal:({type})=>console.error(`WS13 gateway fatal: ${type}`)});
  try {await candidate.start();gateway=candidate;await writeFile(portFile,String(port));break;}
  catch(error){await candidate.close();if(error.code!=='EADDRINUSE')throw error;}
}
if(!gateway) throw new Error('WS13 gateway range is occupied');
const stop=async()=>{await gateway.close();process.exit(0);};
process.once('SIGINT',stop);process.once('SIGTERM',stop);
