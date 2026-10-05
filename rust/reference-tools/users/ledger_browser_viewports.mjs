// Original resize_to values describe WebDriver's outer window. The managed
// native probe records the corresponding actual inner viewport for this run.
import fs from 'node:fs'
const runLabels=JSON.parse(fs.readFileSync(process.env.WS11UI_BROWSER_LABELS,'utf8'))
export function viewportFor(labels,width,height){
 const actual=labels['original.window_viewports']?.[`${width}x${height}`]
 if(!actual)throw Error(`Missing measured original window ${width}x${height}`)
 return {width:actual.width,height:actual.height}
}
export async function resizeOriginal(page,width,height){
 await page.setViewportSize(viewportFor(runLabels,width,height))
}
