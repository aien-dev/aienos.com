import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

# Override for a repository checkout: sibling gateway adapter outside science/.
import os
path=Path(os.environ.get('ALLEN_SCIENCE_ADAPTER',str(Path(__file__).parent.parent/'allen_science.py')))
spec=importlib.util.spec_from_file_location('gateway_science_adapter',path)
adapter=importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)

class AdapterControls(unittest.TestCase):
    def fixture(self):
        return {'events':[{'event':'pane.open','data':{'v':1,'id':'p_a','kind':'math','title':'Math'}},
            {'event':'pane.update','data':{'v':1,'id':'p_a','revision':1,'content':{'text':'symbolic result'},
              'provenance':{'status':'unverified','engine':'','expression':'','result':'','source':'','commit':''}}}], 'result':'Result text'}
    def invoke(self, payload):
        with patch.object(adapter.http.client,'HTTPConnection') as conn:
            response=conn.return_value.getresponse.return_value
            response.status=200
            response.read.return_value=json.dumps(payload).encode()
            out=adapter.call('math_compute',{'title':'Math','expression':'Sin[x]'})
            conn.assert_called_once_with('127.0.0.1',18113,timeout=15)
            conn.return_value.close.assert_called_once()
            return out
    def test_positive_fixed_loopback(self):
        good=self.fixture()
        self.assertEqual(self.invoke(good),good)
    def test_authority_events_refused(self):
        for name in ('human','state','error','done','start','queue'):
            data=self.fixture(); data['events'][0]['event']=name
            with self.subTest(name=name), self.assertRaises(ValueError): self.invoke(data)
    def test_unknown_tool_does_not_connect(self):
        with patch.object(adapter.http.client,'HTTPConnection') as conn:
            with self.assertRaises(ValueError): adapter.call('shell',{'command':'deploy'})
            conn.assert_not_called()
    def test_content_schema_refused(self):
        data=self.fixture(); data['events'][1]['data']['content']['html']='<script>bad</script>'
        with self.assertRaises(ValueError): self.invoke(data)
    def test_update_before_open_refused(self):
        data=self.fixture(); data['events'].reverse()
        with self.assertRaises(ValueError): self.invoke(data)
    def test_output_size_refused(self):
        data=self.fixture(); data['result']='x'*8001
        with self.assertRaises(ValueError): self.invoke(data)
    def test_revision_replay_refused(self):
        data=self.fixture(); data['events'].append(data['events'][1])
        with self.assertRaises(ValueError): self.invoke(data)

if __name__=='__main__': unittest.main(verbosity=2)
