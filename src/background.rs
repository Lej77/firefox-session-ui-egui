use std::fmt;

#[cfg(not(target_family = "wasm"))]
pub fn spawn<F>(fut: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::task::spawn(fut);
}
#[cfg(target_family = "wasm")]
pub fn spawn<F>(fut: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(fut);
}

pub struct EguiBackgroundSender<T>(std::sync::mpsc::Sender<T>);
impl<T> EguiBackgroundSender<T> {
    pub fn send(&self, ctx: &egui::Context, msg: T) {
        if let Err(e) = self.0.send(msg) {
            log::error!("Failed to send message to background thread: {e}");
        } else {
            ctx.request_repaint();
        }
    }
    /// Usually you want [`EguiBackgroundSender::send`] but if you know a
    /// repaint is or will be queued then this method can be more convenient and
    /// more performant.
    #[expect(dead_code, reason = "We aren't currently using this method")]
    pub fn send_without_repaint(&self, msg: T) {
        if let Err(e) = self.0.send(msg) {
            log::error!("Failed to send message to background thread: {e}");
        }
    }
}
impl<T> Clone for EguiBackgroundSender<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> fmt::Debug for EguiBackgroundSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("EguiBackgroundSender")
            .field(&self.0)
            .finish()
    }
}

pub struct EguiBackgroundWork<T> {
    sender: EguiBackgroundSender<T>,
    receiver: std::sync::mpsc::Receiver<T>,
}
impl<T> EguiBackgroundWork<T> {
    pub fn poll_work(&self) -> Option<T> {
        self.receiver.try_recv().ok()
    }

    pub fn sender(&self) -> &EguiBackgroundSender<T> {
        &self.sender
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn spawn<F>(&self, cx: &egui::Context, fut: F)
    where
        F: std::future::Future<Output = Option<T>> + Send + 'static,
        T: Send + 'static,
    {
        let tx = self.sender.clone();
        let cx = cx.clone();
        spawn(async move {
            let Some(msg) = fut.await else { return };
            tx.send(&cx, msg);
        });
    }
    #[cfg(target_family = "wasm")]
    pub fn spawn<F>(&self, cx: &egui::Context, fut: F)
    where
        F: std::future::Future<Output = Option<T>> + 'static,
        T: 'static,
    {
        let tx = self.sender.clone();
        let cx = cx.clone();
        spawn(async move {
            let Some(msg) = fut.await else { return };
            tx.send(&cx, msg);
        });
    }
}
impl<T> Default for EguiBackgroundWork<T> {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            sender: EguiBackgroundSender(tx),
            receiver: rx,
        }
    }
}
