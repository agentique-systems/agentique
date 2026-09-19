"""Independently check original defects and exact corrective reference facts."""
import json
from pathlib import Path
from xmi import Model, XID

OUT=Path(__file__).resolve().parent
old=Model(OUT/'historical/sysml.library.xmi')
new=Model(OUT/'release/sysml.library.xmi',list((OUT/'release/sysml.library.xmi.implied').rglob('*.kermlx')))
report=[]

def members(m,e):return [(r,c) for r in e if m.kind(r).endswith('Membership') for c in r if c.tag=='ownedRelatedElement']
def redefs(m,e):return [t for r in e if m.kind(r)=='Redefinition' for t in m.targets(r,'redefinedFeature')]
def edges(m,e,kind,prop):return [t for r in e if m.kind(r)==kind for t in m.targets(r,prop)]
def record(m,e):return dict(file=m.files[e],id=e.get(XID),path=m.path(e),kind=m.kind(e),xml=m.record(e)['xml'])

for geometric,feature in [('Surface','faces'),('Curve','edges'),('Point','vertices')]:
    root='Objects::StructuredSpaceObject'
    before=old.find(root+'::'+feature)
    assert [old.path(t) for t in edges(old,before,'FeatureTyping','type')]==['Objects::'+geometric]
    assert not any(old.name(c)=='innerSpaceDimension' for _,c in members(old,before))
    conflicts=[old.find(root+'::innerSpaceDimension'),old.find('Objects::'+geometric+'::innerSpaceDimension')]
    assert len(set(conflicts))==2
    combined=new.find(root+'::Structured'+geometric)
    common=new.find(root+'::Structured'+geometric+'::innerSpaceDimension')
    expected={root,'Objects::'+geometric}
    assert {new.path(t) for t in edges(new,combined,'Subclassification','superclassifier')}==expected
    assert {new.path(t) for t in edges(new,combined,'Intersecting','intersectingType')}==expected
    assert {new.path(t) for t in redefs(new,common)}=={old.path(t) for t in conflicts}
    assert edges(new,new.find(root+'::'+feature),'FeatureTyping','type')==[combined]
    report.append(dict(model='Objects',case=feature,original=record(old,before),conflicts=[record(old,t) for t in conflicts],repair=record(new,common),verified=True))

base='FeatureReferencingPerformances::FeatureMonitorPerformance::endWhen'
old_base=old.find(base)
old_ends=[(r,c) for r,c in members(old,old_base) if c.get('isEnd')=='true']
assert len(old_ends)==2
assert [old.path(t) for t in edges(old,old_base,'FeatureTyping','type')]==['Occurrences::HappensBefore']
for number in [1,2]:
    path=f'FeatureReferencingPerformances::BooleanEvaluationResultToMonitorPerformance::monitor{number}::endWhen'
    before=old.find(path)
    assert redefs(old,before)==[old_base]
    assert [old.path(t) for t in edges(old,before,'FeatureTyping','type')]==['Occurrences::HappensJustBefore']
    assert not members(old,before)
    after=new.find(path)
    introduced=[c for _,c in members(new,after) if c.get('declaredName') in ['earlierOccurrence','laterOccurrence']]
    assert [new.name(e) for e in introduced]==['earlierOccurrence','laterOccurrence']
    for index,(name,common) in enumerate(zip(['earlierOccurrence','laterOccurrence'],introduced)):
        other=old.find('Occurrences::HappensJustBefore::'+name)
        ancestor=old.find('Occurrences::HappensBefore::'+name)
        assert redefs(old,other)==[ancestor]
        # The old unnamed EndFeatureMembership acquires the ancestor's name
        # and redefinition at this owned position. The competing specialization
        # owns a distinct end that redefines that same ancestor.
        assert old_ends[index][1] is not other
        assert {new.path(t) for t in redefs(new,common)}=={base+'::'+name,'Occurrences::HappensJustBefore::'+name}
        assert common.get('isEnd')=='true'
        report.append(dict(model='FeatureReferencingPerformances',case=path+'::'+name,
            original=record(old,before),conflicts=[record(old,old_ends[index][1]),record(old,other)],
            common_ancestor=record(old,ancestor),positional_rule='checkFeatureEndRedefinition, owned positions 1 and 2',repair=record(new,common),verified=True))

evaluation='FeatureReferencingPerformances::EvaluationResultMonitorPerformance::onOccurrence::monitoredOccurrence'
boolean='FeatureReferencingPerformances::BooleanEvaluationResultMonitorPerformance::onOccurrence::monitoredOccurrence'
assert old.kind(old.find(evaluation))=='Feature'
assert new.kind(new.find(evaluation))=='Expression'
old_result=old.find(evaluation+'::result')
assert old.kind(old.parents[old_result])=='FeatureMembership'
assert new.kind(new.parents[new.find(evaluation+'::result')])=='ReturnParameterMembership'
common=new.find(boolean+'::result')
assert {new.path(t) for t in redefs(new,common)}=={evaluation+'::result','Performances::BooleanEvaluation::result'}
assert common.get('direction')=='out'
assert 'ScalarValues::Boolean' in {new.path(t) for t in edges(new,common,'FeatureTyping','type')}
report.append(dict(model='FeatureReferencingPerformances',case=boolean+'::result',original=record(old,old.find(boolean)),
    conflicts=[record(old,old_result),record(old,next(c for r,c in members(old,old.find('Performances::BooleanEvaluation')) if old.kind(r)=='ReturnParameterMembership'))],repair=record(new,common),verified=True))

path='Observation::ObserveChange::transfer'
before=old.find(path);after=new.find(path)
assert not any(old.name(c)=='target' for _,c in members(old,before))
common=new.find(path+'::target')
assert common.get('isEnd')=='true'
targets={new.path(t) for t in redefs(new,common)}
assert targets=={'Transfers::TransferBefore::target','Occurrences::Occurrence::outgoingTransfersFromSelf::target','Occurrences::Occurrence::incomingTransfers::target'}
report.append(dict(model='Observation',case=path+'::target',original=record(old,before),repair=record(new,common),target_paths=sorted(targets),verified=True))

path='VectorFunctions::CartesianThreeVectorOf'
before=next(c for r,c in members(old,old.find(path)) if old.kind(r)=='ReturnParameterMembership')
assert not any(old.kind(r)=='FeatureMembership' for r,c in members(old,before))
after=next(c for r,c in members(new,new.find(path)) if new.kind(r)=='ReturnParameterMembership')
common=next(c for _,c in members(new,after) if len(redefs(new,c))==2)
targets=redefs(new,common)
# The source qualifies the inherited member through CartesianThreeVectorValue;
# its canonical declaring owner in XMI is ThreeVectorValue, not a copied member.
assert {new.path(t) for t in targets}=={'VectorFunctions::CartesianVectorOf::result::dimension','VectorValues::ThreeVectorValue::dimension'}
report.append(dict(model='VectorFunctions',case=path+'::<return>::dimension',original=record(old,before),repair=record(new,common),target_paths=[new.path(t) for t in targets],verified=True))

assert len(report)==10
assert {r['model'] for r in report}=={'Objects','FeatureReferencingPerformances','Observation','VectorFunctions'}
(OUT/'four-model-witnesses.json').write_text(json.dumps(dict(verified=True,scope='Source/XMI structural repair facts; independent of Agentique patch helpers. Actual full corpus results remain a separate gate.',cases=report),indent=2)+'\n')
print('Verified all 10 reviewed collision patterns across all four named models')
