use super::{Job, JobResponse, WorkerDetail, WorkerInfo, WorkerState};
use crate::db::{AppEvent, DbPool, HWKeyUserData, User};
use std::{
    collections::hash_map::{Entry, HashMap},
    env,
    net::{IpAddr, Ipv4Addr},
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Request, Response, Status};

tonic::include_proto!("worker");

impl WorkerInfo {
    /// Create new `Worker` instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_seen: Instant::now(),
            ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            jobs: Vec::new(),
        }
    }

    /// Update connectivity timer.
    pub fn refresh_status(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Connectivity status.
    pub fn connected(&self) -> bool {
        self.last_seen.elapsed().as_secs() < 2
    }

    /// Return first availale Job.
    pub fn get_job(&self) -> Option<&Job> {
        self.jobs.first()
    }

    /// Set worker ip
    pub fn set_ip(&mut self, ip: IpAddr) {
        self.ip = ip;
    }

    /// Add Job.
    pub fn add_job(&mut self, job: Job) {
        self.jobs.push(job);
    }

    /// Remove Job with given id.
    pub fn remove_job_with_id(&mut self, job_id: u32) -> Option<Job> {
        if let Some(index) = self.jobs.iter().position(|job| job.id == job_id) {
            Some(self.jobs.remove(index))
        } else {
            None
        }
    }
}

impl Default for WorkerInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerState {
    /// Return initial state.
    #[must_use]
    pub fn new(webhook_tx: UnboundedSender<AppEvent>) -> Self {
        Self {
            current_job_id: 1,
            workers: HashMap::new(),
            job_status: HashMap::new(),
            webhook_tx,
        }
    }

    /// Return `true` on success.
    pub fn register_worker(&mut self, id: String) -> bool {
        if let Entry::Vacant(entry) = self.workers.entry(id) {
            entry.insert(WorkerInfo::new());
            true
        } else {
            false
        }
    }

    /// Create a new job.
    /// Return job id.
    pub fn create_job(
        &mut self,
        worker_id: &str,
        first_name: String,
        last_name: String,
        email: String,
        username: String,
    ) -> u32 {
        if let Some(worker) = self.workers.get_mut(worker_id) {
            let id = self.current_job_id;
            self.current_job_id = id.wrapping_add(1);
            worker.add_job(Job {
                id,
                first_name,
                last_name,
                email,
                username,
            });
            id
        } else {
            0
        }
    }

    /// Remove a job for a given worker.
    pub fn remove_job(&mut self, id: &str, job_id: u32) -> Option<Job> {
        if let Some(worker) = self.workers.get_mut(id) {
            worker.refresh_status();
            worker.remove_job_with_id(job_id)
        } else {
            None
        }
    }

    /// Return the first available job.
    pub fn get_job(&mut self, id: &str, ip: IpAddr) -> Option<&Job> {
        if let Some(worker) = self.workers.get_mut(id) {
            worker.refresh_status();
            worker.set_ip(ip);
            worker.get_job()
        } else {
            None
        }
    }

    pub fn list_workers(&self) -> Vec<WorkerDetail> {
        let mut w = Vec::new();
        for (id, worker) in &self.workers {
            let workers = WorkerDetail {
                id: id.clone(),
                ip: worker.ip,
                connected: worker.connected(),
            };
            w.push(workers);
        }
        w
    }

    pub fn remove_worker(&mut self, id: &str) -> bool {
        self.workers.remove_entry(id).is_some()
    }

    pub fn set_job_status(
        &mut self,
        job_id: u32,
        success: bool,
        pgp_key: String,
        pgp_cert_id: String,
        ssh_key: String,
        error: String,
    ) {
        self.job_status.insert(
            job_id,
            JobResponse {
                success,
                pgp_key,
                pgp_cert_id,
                ssh_key,
                error,
            },
        );
    }

    pub fn get_job_status(&self, job_id: u32) -> Option<&JobResponse> {
        self.job_status.get(&job_id)
    }
}

pub struct WorkerServer {
    pool: DbPool,
    state: Arc<Mutex<WorkerState>>,
}

impl WorkerServer {
    #[must_use]
    pub fn new(pool: DbPool, state: Arc<Mutex<WorkerState>>) -> Self {
        Self { pool, state }
    }
}

#[tonic::async_trait]
impl worker_service_server::WorkerService for WorkerServer {
    async fn register_worker(&self, request: Request<Worker>) -> Result<Response<()>, Status> {
        let message = request.into_inner();
        let mut state = self.state.lock().unwrap();
        if state.register_worker(String::from(&message.id)) {
            debug!("Added worker with id: {}", message.id);
            Ok(Response::new(()))
        } else {
            Err(Status::already_exists("Worker already registered"))
        }
    }

    async fn get_job(&self, request: Request<Worker>) -> Result<Response<GetJobResponse>, Status> {
        let ip = request
            .remote_addr()
            .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |addr| addr.ip());
        let message = request.into_inner();
        let mut state = self.state.lock().unwrap();
        if let Some(job) = state.get_job(&message.id, ip) {
            Ok(Response::new(GetJobResponse {
                first_name: job.first_name.clone(),
                last_name: job.last_name.clone(),
                email: job.email.clone(),
                job_id: job.id,
            }))
        } else {
            Err(Status::not_found("No more jobs"))
        }
    }

    async fn set_job_done(&self, request: Request<JobStatus>) -> Result<Response<()>, Status> {
        let message = request.into_inner();
        // Remove job and set status
        if let Some(job_done) = {
            let mut state = self.state.lock().unwrap();
            state.set_job_status(
                message.job_id,
                message.success,
                message.public_key.clone(),
                message.fingerprint.clone(),
                message.ssh_key.clone(),
                message.error,
            );
            #[allow(clippy::let_and_return)]
            let job = state.remove_job(&message.id, message.job_id);
            job
        } {
            if message.success {
                {
                    // FIXME: locked again
                    let state = self.state.lock().unwrap();
                    let _ = state
                        .webhook_tx
                        .send(AppEvent::HWKeyProvision(HWKeyUserData {
                            username: job_done.username.clone(),
                            email: job_done.email.clone(),
                            ssh_key: message.ssh_key.clone(),
                            pgp_key: message.public_key.clone(),
                            pgp_cert_id: message.fingerprint.clone(),
                        }));
                }
                match User::find_by_username(&self.pool, &job_done.username).await {
                    Ok(Some(mut user)) => {
                        user.ssh_key = Some(message.ssh_key);
                        user.pgp_key = Some(message.public_key);
                        user.pgp_cert_id = Some(message.fingerprint);
                        user.save(&self.pool)
                            .await
                            .map_err(|_| Status::internal("database error"))?;
                    }
                    Ok(None) => info!("User {} not found", job_done.username),
                    Err(err) => error!("Error {}", err),
                }
            }
        }
        Ok(Response::new(()))
    }
}
