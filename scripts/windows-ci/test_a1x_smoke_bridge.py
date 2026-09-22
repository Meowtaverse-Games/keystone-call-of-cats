import hashlib, importlib.util, json, tempfile, unittest
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
 def test_pull_request_head_sha_is_informational(self):
  original=b.gh_json; b.gh_json=lambda *x:{'repository':{'full_name':'o/r'},'path':b.WORKFLOW,'conclusion':'success','head_sha':'old'}
  try:
   self.assertEqual(b.ensure_run('o/r','1','new')['head_sha'], 'old')
  finally: b.gh_json=original
 def test_failed_completed_job_rejected(self):
  with tempfile.TemporaryDirectory() as temp:
   d=Path(temp); (d/'result.zip').write_bytes(b'x')
   (d/'result.json').write_text(json.dumps({'id':'job','state':'failed','sha256':hashlib.sha256(b'x').hexdigest()}))
   with self.assertRaises(ValueError): b.verify_result({'id':'job','directory':str(d)},None)
if __name__=='__main__': unittest.main()
