// Rails' evaluate_async_script completes each sample before testing its result.
export async function pollBrowser(page, predicate, arg, {timeout, polling, message}) {
  const deadline=performance.now()+timeout;
  for (;;) {
    const remaining=deadline-performance.now();
    if(remaining<=0) throw new Error(message+': Timeout '+timeout+'ms exceeded');
    let timer;
    const result=await Promise.race([
      page.evaluate(predicate,arg),
      new Promise((_,reject)=>{timer=setTimeout(()=>reject(new Error(message+': Timeout '+timeout+'ms exceeded')),remaining);})
    ]).finally(()=>clearTimeout(timer));
    if(result) return result;
    await new Promise(resolve=>setTimeout(resolve,Math.min(polling,Math.max(0,deadline-performance.now()))));
  }
}
