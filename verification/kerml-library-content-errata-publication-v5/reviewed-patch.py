"""Materialize the individually reviewed semantic delta from pinned selectors.

Maintenance only: this does not parse replacement source or execute at build time.
The runtime consumes the frozen manifest; independent verification reads its facts.
"""
import hashlib
import json
from pathlib import Path
import re
import uuid
from evidence_io import read_json

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
baseline = read_json(OUT / 'declarations-3/model.json')
records = baseline['records']
selectors = {}
classes = {e['metaclass']: e['metaclass_id'] for e in records.values()}
constants = (ROOT / 'crates/kerml/src/generated/typed_views.rs').read_text()
props = {name: str(uuid.UUID(int=int(value,16))) for name,value in re.findall(r'pub const (\w+): agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128\(0x([0-9a-f_]+)\)',constants)}


def pinned(key, identifier):
    e = records[identifier]
    s = e['source']
    selectors[key] = dict(id=identifier, document=s['document'], sha256=s['sha256'], range=s['range'], metaclass=e['metaclass_id'])
    return dict(pinned=key)


def output(key):
    return dict(output=key)


def by_name(document, name, kind):
    matches = [id for id,e in records.items() if e['metaclass']==kind and e['source']['document'].endswith('/'+document+'.kerml')
               and any(s['name']=='declaredName' and s['value']=='Scalar(String('+json.dumps(name)+'))' for s in e['slots'].values())]
    assert len(matches)==1,(document,name,matches)
    return pinned(document+'::'+name,matches[0])


def owned(identifier, kind):
    ids = next(s['references'] for s in records[identifier]['slots'].values() if s['name']=='ownedRelationship')
    matches = [i for i in ids if records[i]['metaclass']==kind]
    assert len(matches)==1,(identifier,kind,matches)
    return matches[0]


entries=[]


def entry(number, document):
    e=dict(id='AGQ-KERML10-LIB-'+number,document=document,authority=[
        'https://issues.omg.org/issues/KERML11-76',
        'https://github.com/Systems-Modeling/SysML-v2-Pilot-Implementation/commit/240fba1faea614d6aa7f905cc697b95b8bf68d04',
    ],operations=[])
    entries.append(e)
    return e


def put(e, subject, property, value, role=None):
    key=role or next(iter(subject.values()))+'/'+property.lower()
    e['operations'].append(dict(operation='set',key=key,element=subject,property=props[property],value=value))


def append(e,subject,property,target,role):
    e['operations'].append(dict(operation='append',key=role,element=subject,property=props[property],target=target))


def create(e,key,kind):
    e['operations'].append(dict(operation='create',key=key,metaclass=classes[kind]))
    # The same exact pinned metamodel defaults as source lowering, explicitly
    # recorded as correction facts, with no invented source locations.
    example=next(r for r in records.values() if r['metaclass']==kind)
    for pid,s in example['slots'].items():
        if s['value'].startswith('Scalar(Boolean('):
            flag=s['name']=='isUnique'
            e['operations'].append(dict(operation='set',key=key+'/'+s['name'],element=output(key),property=pid,value=dict(boolean=flag)))
    if kind.endswith('Membership'):
        example=next(r for r in records.values() if r['metaclass']==kind and any(s['name']=='visibility' for s in r['slots'].values()))
        visibility=next(s for s in example['slots'].values() if s['name']=='visibility')
        # Select the exact public literal from the generated descriptor table.
        public=re.search(r'\(EnumerationLiteralId::from_u128\(0x([0-9a-f_]+)\), "public"', (ROOT/'crates/kerml/src/generated/complete.rs').read_text())
        assert public
        put(e,output(key),'MEMBERSHIP_VISIBILITY',dict(enumeration=str(uuid.UUID(int=int(public[1],16)))))
    return output(key)


def member(e,key,owner,kind='Feature',membership='FeatureMembership',name=None):
    m=create(e,key+'/membership',membership)
    f=create(e,key,kind)
    append(e,owner,'ELEMENT_OWNED_RELATIONSHIP',m,key+'/owner')
    append(e,m,'RELATIONSHIP_OWNED_RELATED_ELEMENT',f,key+'/member')
    if name is not None:put(e,f,'ELEMENT_DECLARED_NAME',dict(string=name))
    return f


def relation(e,key,owner,kind,target):
    r=create(e,key,kind)
    append(e,owner,'ELEMENT_OWNED_RELATIONSHIP',r,key+'/owner')
    specifics={'Subclassification':'SUBCLASSIFICATION_SUBCLASSIFIER','Redefinition':'REDEFINITION_REDEFINING_FEATURE','FeatureTyping':'FEATURE_TYPING_TYPED_FEATURE'}
    generals={'Subclassification':'SUBCLASSIFICATION_SUPERCLASSIFIER','Redefinition':'REDEFINITION_REDEFINED_FEATURE','FeatureTyping':'FEATURE_TYPING_TYPE','Intersecting':'INTERSECTING_INTERSECTING_TYPE'}
    if kind in specifics:put(e,r,specifics[kind],dict(reference=owner))
    put(e,r,generals[kind],dict(reference=target))


e=entry('001','Kernel Semantic Library/Objects.kerml')
e['authority'].extend([
    'https://github.com/Systems-Modeling/SysML-v2-Pilot-Implementation/commit/4aa78ea896dec76d62d62478534cd9cc6d8217cc',
    'https://github.com/Systems-Modeling/SysML-v2-Pilot-Implementation/commit/692849294651ad8031f25f4323d38e51acb0a1dc',
])
owner=pinned('Objects::StructuredSpaceObject','03a7589e-3bb2-59c2-8303-c541d5d61516')
common=pinned('Objects::StructuredSpaceObject::innerSpaceDimension','fadfd4f6-26b1-50b5-bb0c-406987627413')
for general,feature,dimension,specific in [
    ('Surface','faces','056a9190-f8d8-55fa-a0d6-e74b3b4570b7','6f8ea8fd-31ac-590a-b94d-5d5b9ace0ecf'),
    ('Curve','edges','ab28cdfe-f757-5e2c-abbb-20904c82920f','172ffcc9-142d-5b0c-ab6c-1ba878491506'),
    ('Point','vertices','2b1f871b-4f04-5d9e-a613-418a204ae412','59e9dfaf-0dda-5e9a-a24b-77781a3532e4')]:
    key='Structured'+general
    combined=member(e,key,owner,'Structure','OwningMembership',key)
    other=by_name('Objects',general,'Structure')
    for role,target in [('structured',owner),('geometric',other)]:
        relation(e,key+'/'+role,combined,'Subclassification',target)
        relation(e,key+'/intersection-'+role,combined,'Intersecting',target)
    f=member(e,key+'/innerSpaceDimension',combined)
    relation(e,key+'/common-redefinition',f,'Redefinition',common)
    relation(e,key+'/geometric-redefinition',f,'Redefinition',pinned('Objects::'+general+'::innerSpaceDimension',dimension))
    typing=pinned('Objects::StructuredSpaceObject::'+feature+'/typing',owned(specific,'FeatureTyping'))
    put(e,typing,'FEATURE_TYPING_TYPE',dict(reference=combined))

e=entry('002','Kernel Semantic Library/FeatureReferencingPerformances.kerml')
promoted=pinned('EvaluationMonitor/monitoredOccurrence','c85eefa6-a152-5060-91b6-f6a543cae2ec')
e['operations'].append(dict(operation='reclassify',key='EvaluationMonitor/expression',element=promoted,metaclass=classes['Expression']))
result='f7074ed9-3549-5d73-ad5e-2bf54c5ccc06'
members=[i for i,r in records.items() if r['metaclass']=='FeatureMembership' and any(result in s['references'] for s in r['slots'].values())]
assert len(members)==1
e['operations'].append(dict(operation='reclassify',key='EvaluationMonitor/result-membership',element=pinned('EvaluationMonitor/result-membership',members[0]),metaclass=classes['ReturnParameterMembership']))
boolean=pinned('BooleanEvaluationMonitor/monitoredOccurrence','a39fa9f8-e0e8-5964-b866-938d7cdbd4bd')
result=member(e,'BooleanEvaluationMonitor/result',boolean,membership='ReturnParameterMembership',name='result')
example=records['6239c627-2efd-50f2-bf4f-c8c6cf5233b8']
direction=next(s['value'] for s in example['slots'].values() if s['name']=='direction')
put(e,result,'FEATURE_DIRECTION',dict(enumeration=str(uuid.UUID(int=int(re.search(r'\((\d+)\)',direction)[1])))))
relation(e,'BooleanEvaluationMonitor/result/type',result,'FeatureTyping',by_name('ScalarValues','Boolean','DataType'))
multiplicity=member(e,'BooleanEvaluationMonitor/result/multiplicity',result,'MultiplicityRange','OwningMembership')
bound=member(e,'BooleanEvaluationMonitor/result/multiplicity/bound',multiplicity,'LiteralInteger','OwningMembership')
put(e,bound,'LITERAL_INTEGER_VALUE',dict(integer='1'))
bound_result=member(e,'BooleanEvaluationMonitor/result/multiplicity/bound/result',bound,membership='ReturnParameterMembership')
put(e,bound_result,'FEATURE_DIRECTION',dict(enumeration=str(uuid.UUID(int=int(re.search(r'\((\d+)\)',direction)[1])))))
for n,identifier in [(1,'333ba226-0b0f-5992-97a3-e42c5135efa2'),(2,'6ac03b22-5656-5498-8a39-83b81e60094d')]:
    owner=pinned(f'Monitor{n}/endWhen',identifier)
    for name in ['earlierOccurrence','laterOccurrence']:
        f=member(e,f'Monitor{n}/{name}',owner,name=name)
        # Set the created feature's explicit end flag once, replacing the default
        # operation in this reviewed recipe (not a runtime overwrite).
        operation=next(o for o in e['operations'] if o['key']==f'Monitor{n}/{name}/isEnd')
        operation['value']={'boolean':True}

e=entry('003','Kernel Semantic Library/Observation.kerml')
owner=pinned('Observation::ObserveChange::transfer','0597cd26-c12e-52b5-af35-5c3dc4332452')
member(e,'transfer/target',owner,name='target')
next(o for o in e['operations'] if o['key']=='transfer/target/isEnd')['value']={'boolean':True}

e=entry('004','Kernel Function Library/VectorFunctions.kerml')
owner=pinned('VectorFunctions::CartesianThreeVectorOf/result','6239c627-2efd-50f2-bf4f-c8c6cf5233b8')
dimension=member(e,'CartesianThreeVectorOf/result/dimension',owner)
for role,id in [('constructed','1dd35c9b-e819-546c-926d-a85dfc883be2'),('three','a5bfbc40-b073-5449-a8a9-b62b57ee3235')]:
    relation(e,'CartesianThreeVectorOf/result/dimension/'+role,dimension,'Redefinition',pinned('VectorDimension/'+role,id))

manifest=dict(format='agentique-reviewed-operational-library-corrections/1',profile_id='agentique-kerml-1.0-operational/3',
    library_set=baseline['library_set'],extends=dict(profile='agentique-kerml-1.0-operational/2',
        manifest='standards/kerml-1.0-operational-errata-v2.json',sha256=hashlib.sha256((ROOT/'standards/kerml-1.0-operational-errata-v2.json').read_bytes()).hexdigest()),
    official_issue='KERML11-76',issue_status='open',authority='Agentique project-reviewed operational correction; not an adopted OMG correction',
    selectors=selectors,entries=entries)
path=ROOT/'standards/kerml-1.0-operational-library-errata-v3.json'
path.write_text(json.dumps(manifest,indent=2)+'\n',newline='\n')
print(len(selectors),'selectors;',sum(len(e['operations']) for e in entries),'operations')
