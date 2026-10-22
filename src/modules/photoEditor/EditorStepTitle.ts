import{text}from'../../shared/i18n/Runtime';
import{editorOperations,type EditorStep}from'./EditorContracts';
/** Translate generated labels; preserve names supplied by the user or imported project. */
export function editorStepTitle(step:EditorStep){
 const generated=editorOperations.find(op=>op.id===step.operation);
 if(generated&&(step.title===step.operation||step.title.startsWith(step.operation+' · ')))return generated.label+step.title.slice(step.operation.length);
 const titles={'Исходник':'tree.original','Обрезка':'tree.crop','Изменение размера':'tree.resize','Отражение по горизонтали':'tree.flipHorizontal','Отражение по вертикали':'tree.flipVertical','Добавленный кадр':'tree.import'}as const;const key=titles[step.title as keyof typeof titles];
 return key?text('editor',key):step.title;
}
