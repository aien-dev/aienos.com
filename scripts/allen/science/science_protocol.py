"""Allen pane v1 / compute-call validation. Host JSON adapter; see DEPENDENCIES."""
import json
import math
import re
from urllib.parse import urlsplit

VERSION = 1
MAX_BYTES = 65536
MAX_TEXT = 32768
KINDS = ('math', 'plot', 'doc', 'code', 'app', 'research', 'data', 'omega')
TOOLS = ('math_compute', 'omega_compile', 'web_app', 'paper', 'research', 'plot')
# Positive admission: no contexts, assignments, slots, patterns, shell escapes,
# comments or dynamically assembled symbols. Strings are unit names only.
MATH_HEADS = frozenset('Abs ArcCos ArcSin ArcTan Cos Sin Tan Cosh Sinh Tanh Exp Log Sqrt Power Rational Complex N D Integrate NIntegrate Limit Series Normal Sum Product Solve Reduce Simplify FullSimplify Expand Factor Together Apart Cancel Numerator Denominator Coefficient CoefficientList Collect Det Inverse Transpose Eigenvalues Eigenvectors MatrixRank RowReduce LinearSolve Dot IdentityMatrix DiagonalMatrix Tr EigenSystem Eigensystem DSolve DSolveValue NDSolve Mean Median Variance StandardDeviation Covariance Correlation PDF CDF Quantile NormalDistribution BinomialDistribution PoissonDistribution Gamma Beta Erf Erfc Factorial Binomial GCD LCM Mod Min Max Floor Ceiling Round Sign Re Im Conjugate Chop Length Total Range Table List Plus Times Equal Less Greater LessEqual GreaterEqual And Or Not True False Pi E I Infinity DirectedInfinity'.split())
MATH_HEADS |= frozenset('Assumptions GenerateConditions ConditionalExpression SingularValueList MatrixExp First HermiteH C Quantity QuantityMagnitude QuantityUnit UnitConvert UnitSimplify Probability Expectation Zeta SeriesData'.split())
UNITS = frozenset('Kilograms SpeedOfLight Joules GravitationalConstant SolarMass Kilometers Parsecs LightYears ReducedPlanckConstant Meters Seconds Kelvins Celsius Fahrenheit Radians Degrees Moles Amperes Candelas Newtons Pascals Watts Hertz MetersPerSecond SIBase'.split())
VARIABLES = frozenset('x y z t a b c d f g h k m n p q r s u v w'.split())
TOKEN = re.compile(r'\s+|"[A-Za-z]{1,64}"|\\\[Distributed\]|(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?|[A-Za-z]+|[\[\]{}(),.+*/^<>=!\x27\-]')
WORD = re.compile(r'[A-Za-z]+')


def packed(value):
    return json.dumps(value, ensure_ascii=False, allow_nan=False, separators=(',', ':'))


def obj(value, required, optional=()):
    if type(value) is not dict or set(value) - set(required) - set(optional) or not set(required) <= set(value):
        raise ValueError('Unexpected or missing fields')
    return value


def text(value, limit=MAX_TEXT, empty=True):
    if not isinstance(value, str) or len(value.encode('utf-8')) > limit or (not empty and not value.strip()) or '\x00' in value:
        raise ValueError('Invalid text or text too long')
    return value


def strings(value):
    if type(value) is not list or len(value) > 64:
        raise ValueError('Invalid list')
    return [text(v) for v in value]


def citations(value):
    if type(value) is not list or len(value) > 64:
        raise ValueError('Invalid citations')
    for row in value:
        obj(row, ('title', 'url'))
        text(row['title'], 200, False)
        url = text(row['url'], 2048)
        parsed = urlsplit(url)
        if parsed.scheme != 'https' or not parsed.hostname or parsed.username or parsed.password or any(ord(c) < 33 for c in url):
            raise ValueError('Citation must be a plain HTTPS URL; never fetched by tools')
    return value


def expression(value):
    value = text(value, 4096, False)
    at = 0
    nesting = []
    for match in TOKEN.finditer(value):
        if match.start() != at:
            raise ValueError('Expression contains disallowed syntax')
        token = match.group()
        at = match.end()
        if token.startswith('"') and token[1:-1] not in UNITS:
            raise ValueError('Only approved unit strings are accepted')
        if token == ',' and not nesting:
            raise ValueError('Exactly one expression is required')
        if token[0].isalpha() and token not in MATH_HEADS and token not in VARIABLES:
            raise ValueError('Unsupported mathematical symbol: ' + token)
        if token in ('[', '{', '('):
            nesting.append(token)
            if len(nesting) > 32:
                raise ValueError('Expression nesting limit')
        elif token in (']', '}', ')'):
            if not nesting or nesting.pop() != {']':'[', '}':'{', ')':'('}[token]:
                raise ValueError('Unbalanced expression')
    if at != len(value) or nesting or any(s in value for s in ('==>', '!=', '<>', '!!', '//', '/*', '*/', '(*', '*)')):
        raise ValueError('Expression contains disallowed syntax')
    if re.search(r'(?<![=<>])=(?!=)', value):
        raise ValueError('Assignments are not mathematical requests')
    if '++' in value or '--' in value:
        raise ValueError('Mutation operators are not mathematical requests')
    # Dependent variables may occur as heads in differential equations.
    # A head is a maximal letter run followed, after optional whitespace, by '['.
    # Scanned by hand: the regex form backtracks polynomially on long letter runs.
    for word in WORD.finditer(value):
        after = word.end()
        while after < len(value) and value[after].isspace():
            after += 1
        head = word.group()
        if after < len(value) and value[after] == '[' and head not in MATH_HEADS and head not in VARIABLES and head != 'Distributed':
            raise ValueError('Unapproved function head')
    return value


def validate(name, args):
    if name not in TOOLS:
        raise ValueError('Unknown compute tool')
    if len(packed(args).encode('utf-8')) > MAX_BYTES:
        raise ValueError('Tool arguments too large')
    fields = {'math_compute':('title','expression'), 'omega_compile':('title','source'),
              'web_app':('title','html'), 'paper':('title','markdown','citations'),
              'research':('title','notes','hypotheses','derivations','sources'),
              'plot':('title','series','x_label','y_label')}[name]
    obj(args, fields)
    text(args['title'], 200, False)
    if name == 'math_compute': expression(args['expression'])
    elif name == 'omega_compile': text(args['source'], 16384, False)
    elif name == 'web_app': text(args['html'], MAX_TEXT, False)
    elif name == 'paper':
        text(args['markdown']); citations(args['citations'])
    elif name == 'research':
        text(args['notes']); strings(args['hypotheses']); strings(args['derivations']); citations(args['sources'])
    else:
        text(args['x_label'], 200); text(args['y_label'], 200)
        if type(args['series']) is not list or not 1 <= len(args['series']) <= 8:
            raise ValueError('Invalid series')
        for series in args['series']:
            obj(series, ('label', 'points')); text(series['label'], 200)
            if type(series['points']) is not list or not 1 <= len(series['points']) <= 512:
                raise ValueError('Invalid points')
            for point in series['points']:
                if type(point) is not list or len(point) != 2 or any(type(n) not in (int,float) or not math.isfinite(n) or abs(n) > 1e100 for n in point):
                    raise ValueError('Invalid point')
    return json.loads(packed(args))


def parse_call(value):
    """The only accepted constrained fallback frame; surrounding prose is not executable."""
    if not isinstance(value, str) or len(value.encode('utf-8')) > MAX_BYTES:
        raise ValueError('Invalid tool frame')
    def unique(pairs):
        result = {}
        for key, val in pairs:
            if key in result: raise ValueError('Duplicate JSON key')
            result[key] = val
        return result
    frame = json.loads(value, object_pairs_hook=unique, parse_constant=lambda _: (_ for _ in ()).throw(ValueError('Nonfinite JSON')))
    obj(frame, ('tool', 'arguments'))
    return frame['tool'], validate(frame['tool'], frame['arguments'])


def event(name, data):
    if name not in ('pane.open', 'pane.update', 'pane.close'): raise ValueError('Unknown pane event')
    fields = {'pane.open':('v','id','kind','title'), 'pane.update':('v','id','revision','content','provenance'), 'pane.close':('v','id')}[name]
    obj(data, fields)
    if type(data['v']) is not int or data['v'] != VERSION or not isinstance(data['id'], str) or not re.fullmatch(r'[A-Za-z0-9_-]{1,64}', data['id']):
        raise ValueError('Invalid pane identity')
    if name == 'pane.open':
        if data['kind'] not in KINDS: raise ValueError('Invalid pane kind')
        text(data['title'], 200)
    if name == 'pane.update':
        if type(data['revision']) is not int or not 1 <= data['revision'] <= 1000000: raise ValueError('Invalid revision')
        if type(data['content']) is not dict or len(packed(data['content']).encode()) > MAX_BYTES: raise ValueError('Invalid content')
        p = obj(data['provenance'], ('status','engine','expression','result','source','commit'))
        if p['status'] not in ('computed','unsupported','unverified','input-data'): raise ValueError('Invalid evidence status')
        for key in ('engine','expression','result','source','commit'): text(p[key])
    return data


def content(kind, value):
    """Validate the kind-specific replacement before any SSE event is emitted."""
    if kind == 'math':
        obj(value, ('text',), ('tex',))
        for item in value.values(): text(item)
    elif kind in ('doc', 'research', 'plot', 'app'):
        tool = {'doc':'paper','research':'research','plot':'plot','app':'web_app'}[kind]
        validate(tool, {'title':'Pane', **value})
        if 'title' in value: raise ValueError('Unexpected content title')
    elif kind in ('code', 'omega'):
        obj(value, ('source','language'), ('wasm','functions'))
        text(value['source'], 16384); text(value['language'], 64)
        if 'wasm' in value:
            import base64
            base64.b64decode(text(value['wasm']), validate=True)
        if 'functions' in value:
            if type(value['functions']) is not list or len(value['functions']) > 64: raise ValueError('Invalid function list')
            for function in value['functions']:
                obj(function, ('name','params','ret'))
                text(function['name'],64); text(function['ret'],64)
                if type(function['params']) is not list or len(function['params']) > 6: raise ValueError('Invalid parameters')
                for parameter in function['params']:
                    obj(parameter, ('name','type'))
                    text(parameter['name'],64); text(parameter['type'],64)
    elif kind == 'data':
        obj(value, ('columns','rows'))
        strings(value['columns'])
        if type(value['rows']) is not list or len(value['rows']) > 64: raise ValueError('Invalid rows')
        for row in value['rows']:
            strings(row)
            if len(row) != len(value['columns']): raise ValueError('Invalid row width')
    else:
        raise ValueError('Unknown content kind')
    if len(packed(value).encode()) > MAX_BYTES: raise ValueError('Content too large')
    return value
