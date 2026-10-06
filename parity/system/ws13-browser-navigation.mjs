// Selenium's default navigation strategy waits for document.readyState complete.
export const navigate = (page,url) => page.goto(url,{waitUntil:'load'});
