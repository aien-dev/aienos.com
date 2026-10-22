import datetime,unittest
from renda_metrics import collect
T=datetime.datetime(2026,10,22,20,tzinfo=datetime.timezone.utc).timestamp()
def line(sec,merged=0):
 d=datetime.datetime.fromtimestamp(T+sec,datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
 return f'{d} renda1 pass done in 20 s: merged {merged}, rebased 0, reruns 0, operator runs 0'
class Metrics(unittest.TestCase):
 def test_cycle_includes_rest(self):
  d=collect(line(0)+'\n'+line(80,1),'renda1',T+81);self.assertEqual(d['current_cpm'],.75);self.assertEqual(d['merges'],1);self.assertEqual(d['samples'],1)
 def test_missing_unknown_not_zero(self):self.assertIsNone(collect('','renda1',T)['current_cpm'])
 def test_stale_unknown(self):self.assertIsNone(collect(line(0)+'\n'+line(80),'renda1',T+400)['current_cpm'])
 def test_restart_breaks_cycle(self):self.assertIsNone(collect(line(0)+'\nrenda1 start lanes=x\n'+line(80),'renda1',T+81)['current_cpm'])
 def test_wrong_worker_ignored(self):self.assertEqual(collect(line(0),'renda2',T)['window_passes'],0)
 def test_raw_data_not_exported(self):self.assertNotIn('secret',str(collect('secret /home/private\n'+line(0),'renda1',T)))
 def test_range(self):
  d=collect('\n'.join(line(t) for t in (0,60,180)),'renda1',T+181);self.assertEqual((d['min_cpm'],d['max_cpm'],d['current_cpm']),(.5,1,.5))
 def test_future_ignored(self):self.assertEqual(collect(line(60),'renda1',T)['window_passes'],0)
if __name__=='__main__':unittest.main()
