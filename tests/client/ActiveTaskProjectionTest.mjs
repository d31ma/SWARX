import assert from 'node:assert/strict';
import {test} from 'node:test';
import {OverviewFixtureClient} from './OverviewFixtureClient.mjs';

class ActiveTaskProjectionTest {
  /** @returns {object} */
  static taskOnlyRequest() {
    return {
      now:220,sampled_at:218,started_at:200,
      totals:{cpu_seconds:0,received_bytes:0,sent_bytes:0,disk_read_bytes:0,
        disk_written_bytes:0,peak_resident_bytes:0,resident_bytes:0,samples:1},
      groups:[],activity:[],errors:[],limitations:[],selection:{selected:'',query:'',pid:null},
      logical_agents:[{
        id:'synthetic-task-only-agent',name:'Synthetic task-only fixture',family:'Codex',
        status:'active',source:'Explicit synthetic fixture',captured_at:218,
        feed_mode:'coordinator-report',age_seconds:2,stale:false,reported_status:null,
        metrics:null,executions:[],
        task:{id:'synthetic-task',name:'Synthetic assignment',status:'active',reported_status:null,depends_on:[]},
        session:{id:'synthetic-session',name:'Synthetic session',source:'Explicit synthetic fixture'}
      }]
    };
  }

  /** @returns {object} */
  static emptyRequest() {
    return {...this.taskOnlyRequest(),logical_agents:[]};
  }
}

test('public native projection keeps a fresh active task without a command', /** @returns {void} */ () => {
  const view=OverviewFixtureClient.project(ActiveTaskProjectionTest.taskOnlyRequest());
  assert.equal(view.logicalCount,1);
  assert.equal(view.logical[0].currentAction,null);
  assert.equal(view.logical[0].actionTiming,null);
  assert.deepEqual(view.logical[0].command_resources,[]);
});

test('public native projection explains missing reports without claiming no actual work', /** @returns {void} */ () => {
  const view=OverviewFixtureClient.project(ActiveTaskProjectionTest.emptyRequest());
  assert.equal(view.logicalCount,0);
  assert.match(view.logicalNotice,/does not prove no agents are working/);
});
