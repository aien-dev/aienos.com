import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import allen_science as service
from science_protocol import content


class ServiceControls(unittest.TestCase):
    def test_manifest_identity_bound_to_binary(self):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/'engine.json'
            with patch.object(service,'MANIFEST',path):
                with self.assertRaises(FileNotFoundError): service.load_manifest()
                path.write_text(json.dumps({'commit':'a'*40,'binary_sha256':'b'*64}))
                with self.assertRaises(ValueError): service.load_manifest()
                path.write_text(json.dumps({'commit':'a'*40,'binary_sha256':service.AIEN_MATH_PIN}))
                self.assertEqual(service.load_manifest()['binary_sha256'],service.AIEN_MATH_PIN)
                path.write_text(json.dumps({'commit':'main','binary_sha256':service.AIEN_MATH_PIN}))
                with self.assertRaises(ValueError): service.load_manifest()
    def test_unknown_tool_does_not_execute(self):
        with patch.object(service,'run_math') as run:
            with self.assertRaises(ValueError): service.perform('deploy',{'command':'do it'})
            run.assert_not_called()
    def test_unsupported_never_claims_computed(self):
        with patch.object(service,'run_math',return_value={'ok':False,'output':'Integrate[f[x],x]'}):
            result=service.perform('math_compute',{'title':'Math','expression':'Integrate[f[x],x]'})
        update=result['events'][1]['data']
        self.assertEqual(update['provenance']['status'],'unsupported')
        self.assertIn('NOT SUPPORTED',result['result'])
        self.assertNotIn('Integrate[f[x],x]',update['content']['text'])
        self.assertEqual(update['provenance']['result'],'Integrate[f[x],x]')
    def test_long_result_not_silently_cut(self):
        # Protocol/control fixture, not a math accuracy test.
        raw='a'*8100
        with patch.object(service,'run_math',return_value={'ok':True,'output':raw}):
            result=service.perform('math_compute',{'title':'Math','expression':'Expand[(a+b)^2]'})
        self.assertEqual(result['events'][1]['data']['content']['text'],raw)
        self.assertEqual(result['events'][1]['data']['provenance']['result'],raw)
        self.assertIn('exceeds the model context limit',result['result'])
        self.assertNotIn('exact output',result['result'])
    def test_drafts_are_not_computed(self):
        args={'title':'Paper','markdown':'Draft contains unverified model prose','citations':[]}
        out=service.perform('paper',args)
        self.assertEqual(out['events'][1]['data']['provenance']['status'],'unverified')
        self.assertEqual(out['events'][1]['data']['content']['markdown'],args['markdown'])
    def test_unknown_content_fields_and_bad_wasm(self):
        content('math',{'text':'symbolic result'})
        with self.assertRaises(ValueError): content('math',{'text':'result','html':'<script>bad</script>'})
        with self.assertRaises(ValueError): content('omega',{'source':'ir','language':'omega','wasm':'!!!!'})
        with self.assertRaises(ValueError): content('doc',{'markdown':'draft','citations':[],'title':'extra'})

if __name__=='__main__': unittest.main(verbosity=2)
