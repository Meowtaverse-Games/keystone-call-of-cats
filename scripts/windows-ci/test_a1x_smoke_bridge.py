import importlib.util, unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('bridge',Path(__file__).with_name('a1x-smoke-bridge.py')); b=importlib.util.module_from_spec(spec); spec.loader.exec_module(b)
class BridgeTests(unittest.TestCase):
 def test_offline_worker_rejected(self):
  with self.assertRaises(ValueError): b.require_worker({'workers':[],'jobs':[]})
 def test_active_job_rejected(self):
  with self.assertRaises(ValueError): b.require_worker({'workers':[{'device':'a1x','seen':b.time.time()}],'jobs':[{'device':'a1x','state':'running'}]})
 def test_stale_worker_rejected(self):
  with self.assertRaises(ValueError): b.require_worker({'workers':[{'device':'a1x','seen':0}],'jobs':[]})
 def test_failed_run_rejected(self):
  original=b.gh_json; b.gh_json=lambda *x:{'repository':{'full_name':'o/r'},'path':b.WORKFLOW,'conclusion':'failure','head_sha':'a'}
  try:
   with self.assertRaises(ValueError): b.ensure_run('o/r','1','a')
  finally: b.gh_json=original
if __name__=='__main__': unittest.main()
