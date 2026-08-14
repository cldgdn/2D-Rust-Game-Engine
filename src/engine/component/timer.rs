use crate::engine::component::Tickable;
use crate::engine::EntityID;

/// A simple countup timer (goes from 0.0 up to duration)
/// 
/// This is a gated Tickable ABT.
pub struct Timer {
    duration: f32,
    progress: f32,
    is_running: bool,
    finished: bool,
}

impl Timer {
    pub fn new(duration: f32) -> Self {
        Self {
            duration,
            progress: 0.0,
            is_running: false,
            finished: false,
        }
    }
    
    pub fn is_running(&self) -> bool {
        self.is_running
    }
    /// Returns whether the timer has finished since the last time this was called.
    /// 
    /// Mutability is required as it manually sets its own finished flag to false if true is 
    /// returned, consuming the finished state.
    pub fn finished(&mut self) -> bool {
        let result = self.finished;
        if result {
            self.finished = false;
        }
        
        result
    }
    
    pub fn start(&mut self) {
        self.is_running = true;
        self.finished = false;
        self.progress = 0.0;
    }
    
    pub fn stop(&mut self) {
        self.is_running = false;
    }
    
    pub fn reset(&mut self) {
        self.is_running = false;
        self.progress = 0.0;
    }
    
    pub fn resume(&mut self) {
        self.is_running = true;
    }
}

impl Tickable for Timer {
    /// Ticks up and sets the finished flag if progress reaches duration and resets itself.
    fn tick(&mut self, delta_time: f32) {
        if !self.is_running {
            return;
        }
        
        self.progress += delta_time;
        
        if self.progress > self.duration {
            self.reset();
            self.finished = true;
        }
    }
}