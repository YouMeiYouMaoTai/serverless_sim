use crate::{
    config::Config,
    fn_dag::FnId,
    mechanism::{DownCmd, SimEnvObserve},
    mechanism_thread::{MechCmdDistributor, MechScheduleOnceRes},
    node::NodeId,
    with_env_sub::WithEnvCore,
};

// 原 ScaleExecutor
pub trait ScaleDownExec: Send {
    fn exec_scale_down(
        &mut self,
        sim_env: &SimEnvObserve,
        fnid: FnId,
        scale_cnt: usize,
        cmd_distributor: &MechCmdDistributor,
    ) -> Vec<DownCmd>;

    // /// return success scale up cnt
    // fn scale_up(&mut self, sim_env: &SimEnv, fnid: FnId, scale_cnt: usize) -> usize;
}

pub fn new_scale_down_exec(c: &Config) -> Option<Box<dyn ScaleDownExec>> {
    let es = &c.mech;
    let (scale_down_exec_name, _scale_down_exec_attr) = es.scale_down_exec_conf();

    match &*scale_down_exec_name {
        "default" => {
            return Some(Box::new(DefaultScaleDownExec));
        }
        _ => {
            return None;
        }
    }
}

pub struct DefaultScaleDownExec;

impl DefaultScaleDownExec {
    fn collect_idle_containers(&self, env: &SimEnvObserve) -> Vec<(NodeId, FnId)> {
        let mut idle_container_node_fn = Vec::new();

        for n in env.core().nodes().iter() {
            for (fnid, fn_ct) in n.fn_containers.borrow().iter() {
                if fn_ct.is_idle() {
                    idle_container_node_fn.push((n.node_id(), *fnid));
                }
            }
        }

        idle_container_node_fn
    }

    fn scale_down_for_fn(
        &mut self,
        env: &SimEnvObserve,
        fnid: FnId,
        mut scale_cnt: usize,
        cmd_distributor: &MechCmdDistributor,
    ) -> Vec<DownCmd> {
        let mut collect_idle_containers = self.collect_idle_containers(env);
        collect_idle_containers.retain(|&(_nodeid, fnid_)| fnid_ == fnid);

        if collect_idle_containers.len() < scale_cnt {
            // log::warn!(
            //     "scale down for spec fn {fnid} has failed partly, target:{scale_cnt}, actual:{}",
            //     collect_idle_containers.len()
            // );
            scale_cnt = collect_idle_containers.len();
        }
        let res: Vec<DownCmd> = collect_idle_containers[0..scale_cnt]
            .iter()
            .map(|&(nodeid, fnid)| DownCmd { nid: nodeid, fnid })
            .collect();
        for cmd in res.iter() {
            cmd_distributor
                .send(MechScheduleOnceRes::ScaleDownCmd(cmd.clone()))
                .unwrap();
        }
        res
    }
}

impl ScaleDownExec for DefaultScaleDownExec {
    fn exec_scale_down(
        &mut self,
        env: &SimEnvObserve,
        fnid: FnId,
        scale_cnt: usize,
        cmd_distributor: &MechCmdDistributor,
    ) -> Vec<DownCmd> {
        self.scale_down_for_fn(env, fnid, scale_cnt, cmd_distributor)
    }
}
